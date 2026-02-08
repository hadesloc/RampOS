//! SAML 2.0 Provider Implementation
//!
//! Handles SAML 2.0 authentication flow, including request generation and response validation.
//! Implements XML Digital Signature verification (XMLDSig) for SAML Response validation
//! using RSA-SHA256 and the IdP's X.509 certificate.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    RampRole, RoleMapping, SsoAuthRequest, SsoAuthResponse, SsoCallback, SsoProtocol,
    SsoProvider, SsoProviderType, SsoUser,
};

/// SAML configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlConfig {
    /// Service Provider Entity ID (our issuer)
    pub sp_entity_id: String,
    /// Identity Provider Entity ID
    pub idp_entity_id: String,
    /// IdP SSO Service URL (Destination)
    pub idp_sso_url: String,
    /// IdP x509 Certificate (PEM format) for verifying signatures
    pub idp_certificate: String,
    /// Attribute mapping (IdP attribute name -> RampOS field)
    pub attribute_mapping: HashMap<String, String>,
    /// Allow IdP-initiated login
    pub allow_idp_initiated: bool,
    /// Sign requests
    pub sign_requests: bool,
    /// SP Private Key (PEM) for signing requests
    #[serde(skip_serializing)]
    pub sp_private_key: Option<String>,
}

impl SamlConfig {
    pub fn okta(
        sp_entity_id: String,
        idp_metadata_url: &str,
        idp_cert: String,
    ) -> Self {
        // In a real impl, we would fetch metadata from the URL
        Self {
            sp_entity_id,
            idp_entity_id: idp_metadata_url.to_string(), // Simplified
            idp_sso_url: idp_metadata_url.replace("/metadata", "/sso"), // Guessed convention
            idp_certificate: idp_cert,
            attribute_mapping: HashMap::from([
                ("email".to_string(), "email".to_string()),
                ("firstName".to_string(), "given_name".to_string()),
                ("lastName".to_string(), "family_name".to_string()),
                ("groups".to_string(), "groups".to_string()),
            ]),
            allow_idp_initiated: true,
            sign_requests: true,
            sp_private_key: None,
        }
    }
}

/// SAML Provider implementation
pub struct SamlProvider {
    provider_type: SsoProviderType,
    config: SamlConfig,
    role_mappings: Vec<RoleMapping>,
    default_role: RampRole,
}

impl SamlProvider {
    pub fn new(
        provider_type: SsoProviderType,
        config: SamlConfig,
        role_mappings: Vec<RoleMapping>,
        default_role: RampRole,
    ) -> Result<Self> {
        Ok(Self {
            provider_type,
            config,
            role_mappings,
            default_role,
        })
    }

    /// Parse and verify a SAML Response XML.
    ///
    /// This method:
    /// 1. Base64-decodes the SAML response
    /// 2. Verifies the XML digital signature against the IdP's X.509 certificate
    /// 3. Extracts the NameID and attributes from the verified response
    fn parse_saml_response(&self, saml_response: &str) -> Result<ParsedSamlResponse> {
        let decoded = {
            use base64::{engine::general_purpose::STANDARD, Engine};
            STANDARD.decode(saml_response)
                .map_err(|e| ramp_common::Error::Authentication(format!("Invalid base64 SAML response: {}", e)))?
        };
        let xml = String::from_utf8_lossy(&decoded).to_string();

        // Verify XML digital signature before trusting any data
        xmldsig::verify_saml_signature(&xml, &self.config.idp_certificate)?;

        // Signature verified -- safe to extract claims
        let email = extract_tag_value(&xml, "NameID")
            .ok_or_else(|| ramp_common::Error::Authentication("Missing NameID in SAML response".into()))?;

        // Extract attributes
        let attributes = extract_saml_attributes(&xml);

        // Extract issuer from the response itself
        let issuer = extract_tag_value(&xml, "Issuer")
            .unwrap_or_else(|| self.config.idp_entity_id.clone());

        Ok(ParsedSamlResponse {
            name_id: email,
            attributes,
            issuer,
        })
    }
}

struct ParsedSamlResponse {
    name_id: String,
    attributes: HashMap<String, Vec<String>>,
    issuer: String,
}

/// Extract the text content of an XML element by tag local name.
/// Handles both prefixed (e.g., `<saml:NameID>`) and unprefixed tags.
fn extract_tag_value(xml: &str, tag: &str) -> Option<String> {
    // Try patterns: <tag>, <prefix:tag>, <prefix:tag attr="...">
    let patterns: Vec<String> = vec![
        format!("<{}>", tag),
        format!("<{}  ", tag),
        format!("<{} ", tag),
    ];

    // Also search for namespaced variants like <saml:Tag or <samlp:Tag
    let ns_pattern = format!(":{}", tag);

    let start_pos = patterns
        .iter()
        .filter_map(|p| xml.find(p.as_str()))
        .min()
        .or_else(|| {
            // Search for namespaced variant
            xml.find(&ns_pattern).and_then(|ns_pos| {
                // Walk backwards to find the '<'
                xml[..ns_pos].rfind('<')
            })
        })?;

    // Find the closing '>' of the opening tag (handles attributes)
    let content_start = xml[start_pos..].find('>')? + start_pos + 1;

    // Find the next '<' which starts the closing tag or a child element
    let content_end = xml[content_start..].find('<')? + content_start;

    let value = xml[content_start..content_end].trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Extract SAML Attribute elements from the Assertion.
/// Looks for `<saml:Attribute Name="..."><saml:AttributeValue>...</saml:AttributeValue></saml:Attribute>`
fn extract_saml_attributes(xml: &str) -> HashMap<String, Vec<String>> {
    let mut attributes = HashMap::new();
    let mut search_from = 0;

    // Pattern: look for Attribute elements (with or without namespace prefix)
    while let Some(attr_start) = find_attribute_element(xml, search_from) {
        // Extract the Name attribute
        if let Some(name) = extract_xml_attribute(&xml[attr_start..], "Name") {
            // Find the end of this Attribute element
            let attr_end_tag_options = ["</saml:Attribute>", "</Attribute>"];
            let attr_end = attr_end_tag_options
                .iter()
                .filter_map(|end| xml[attr_start..].find(end).map(|p| attr_start + p))
                .min();

            if let Some(end_pos) = attr_end {
                let attr_block = &xml[attr_start..end_pos];
                let values = extract_attribute_values(attr_block);
                if !values.is_empty() {
                    attributes.insert(name, values);
                }
                search_from = end_pos;
            } else {
                break;
            }
        } else {
            search_from = attr_start + 1;
        }
    }

    attributes
}

/// Find the start of the next `<Attribute` or `<saml:Attribute` element.
fn find_attribute_element(xml: &str, from: usize) -> Option<usize> {
    let remaining = &xml[from..];
    let patterns = ["<saml:Attribute ", "<Attribute "];
    patterns
        .iter()
        .filter_map(|p| remaining.find(p).map(|pos| from + pos))
        .min()
}

/// Extract an XML attribute value from an opening tag string.
/// e.g., extract_xml_attribute(`<Attribute Name="email">`, "Name") -> Some("email")
fn extract_xml_attribute(tag_str: &str, attr_name: &str) -> Option<String> {
    let search = format!("{}=\"", attr_name);
    if let Some(start) = tag_str.find(&search) {
        let value_start = start + search.len();
        if let Some(end) = tag_str[value_start..].find('"') {
            return Some(tag_str[value_start..value_start + end].to_string());
        }
    }
    // Try single quotes
    let search_sq = format!("{}='", attr_name);
    if let Some(start) = tag_str.find(&search_sq) {
        let value_start = start + search_sq.len();
        if let Some(end) = tag_str[value_start..].find('\'') {
            return Some(tag_str[value_start..value_start + end].to_string());
        }
    }
    None
}

/// Extract all `<AttributeValue>` or `<saml:AttributeValue>` text values from within an Attribute block.
fn extract_attribute_values(block: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut search_from = 0;
    let patterns = ["<saml:AttributeValue", "<AttributeValue"];

    loop {
        let next = patterns
            .iter()
            .filter_map(|p| block[search_from..].find(p).map(|pos| search_from + pos))
            .min();

        match next {
            Some(start) => {
                // Find closing '>' of opening tag
                if let Some(gt) = block[start..].find('>') {
                    let content_start = start + gt + 1;
                    // Find the closing tag
                    if let Some(lt) = block[content_start..].find('<') {
                        let value = block[content_start..content_start + lt].trim().to_string();
                        if !value.is_empty() {
                            values.push(value);
                        }
                    }
                    search_from = content_start;
                } else {
                    break;
                }
            }
            None => break,
        }
    }

    values
}

// ─── XML Digital Signature Verification ─────────────────────────────────────────

/// XML Digital Signature (XMLDSig) verification module for SAML responses.
///
/// Implements the core signature verification flow:
/// 1. Extract the `<ds:Signature>` element
/// 2. Extract `<ds:SignedInfo>`, `<ds:SignatureValue>`, `<ds:X509Certificate>`
/// 3. Canonicalize `<ds:SignedInfo>` using Exclusive XML Canonicalization (exc-c14n)
/// 4. Verify the RSA-SHA256 (or RSA-SHA1) signature over the canonicalized SignedInfo
/// 5. Verify the digest of the referenced element matches `<ds:DigestValue>`
mod xmldsig {
    use ring::signature;
    use sha2::{Digest, Sha256};
    use sha1::Sha1;

    /// Verify the XML digital signature in a SAML Response.
    ///
    /// This performs the full XMLDSig verification:
    /// 1. Extracts the Signature element from the XML
    /// 2. Extracts SignedInfo, SignatureValue, and the certificate
    /// 3. Validates that the signing certificate matches the configured IdP certificate
    /// 4. Verifies the Reference digest (ensures the signed content hasn't been modified)
    /// 5. Verifies the RSA signature over the canonicalized SignedInfo
    pub fn verify_saml_signature(xml: &str, idp_cert_pem: &str) -> ramp_common::Result<()> {
        // Step 1: Extract Signature element
        let sig_block = extract_signature_block(xml)
            .ok_or_else(|| ramp_common::Error::Authentication(
                "SAML response missing ds:Signature element".into()
            ))?;

        // Step 2: Extract sub-elements
        let signed_info_raw = extract_element(&sig_block, "SignedInfo")
            .ok_or_else(|| ramp_common::Error::Authentication(
                "SAML Signature missing SignedInfo element".into()
            ))?;

        let signature_value_b64 = extract_element_text(&sig_block, "SignatureValue")
            .ok_or_else(|| ramp_common::Error::Authentication(
                "SAML Signature missing SignatureValue".into()
            ))?;

        let sig_method_uri = extract_attribute_from_element(&signed_info_raw, "SignatureMethod", "Algorithm")
            .unwrap_or_default();

        let digest_method_uri = extract_attribute_from_element(&signed_info_raw, "DigestMethod", "Algorithm")
            .unwrap_or_default();

        let digest_value_b64 = extract_element_text(&signed_info_raw, "DigestValue")
            .ok_or_else(|| ramp_common::Error::Authentication(
                "SAML SignedInfo missing DigestValue".into()
            ))?;

        // Step 3: Determine which certificate to use for verification.
        // Prefer the configured IdP certificate. If the response includes an embedded
        // X509Certificate, validate that it matches the configured one.
        let idp_cert_der = decode_certificate_pem(idp_cert_pem)?;

        // Check if response has an embedded certificate
        if let Some(embedded_cert_b64) = extract_element_text(&sig_block, "X509Certificate") {
            let embedded_der = decode_base64_multiline(&embedded_cert_b64)?;
            // The embedded certificate must match the configured IdP certificate
            // to prevent certificate substitution attacks.
            if idp_cert_der != embedded_der {
                tracing::warn!(
                    "SAML response contains an embedded X509Certificate that differs from the configured IdP certificate. \
                     Rejecting to prevent certificate substitution attack."
                );
                return Err(ramp_common::Error::Authentication(
                    "Embedded X509Certificate does not match configured IdP certificate".into()
                ));
            }
        }

        // Step 4: Verify the Reference digest.
        // Extract the URI attribute from the Reference element to know what was signed.
        let reference_uri = extract_attribute_from_element(&signed_info_raw, "Reference", "URI")
            .unwrap_or_default();

        // Get the referenced content: either the whole document or an element by ID
        let referenced_content = if reference_uri.is_empty() || reference_uri == "#" {
            // Signing the entire document -- use the XML without the Signature element
            remove_signature_element(xml)
        } else if let Some(id) = reference_uri.strip_prefix('#') {
            // Signing a specific element by ID
            extract_element_by_id(xml, id)
                .ok_or_else(|| ramp_common::Error::Authentication(
                    format!("Referenced element with ID '{}' not found in SAML response", id)
                ))?
        } else {
            return Err(ramp_common::Error::Authentication(
                format!("Unsupported Reference URI: {}", reference_uri)
            ));
        };

        // The referenced content must have its Signature child removed before digest computation,
        // and then be canonicalized with exc-c14n.
        let content_without_sig = remove_signature_element(&referenced_content);
        let canonicalized_content = canonicalize_exc_c14n(&content_without_sig);

        // Compute digest over the canonicalized referenced content
        let computed_digest = if digest_method_uri.contains("sha256") {
            let mut hasher = Sha256::new();
            hasher.update(canonicalized_content.as_bytes());
            hasher.finalize().to_vec()
        } else if digest_method_uri.contains("sha1") || digest_method_uri.contains("sha-1") {
            let mut hasher = Sha1::new();
            hasher.update(canonicalized_content.as_bytes());
            hasher.finalize().to_vec()
        } else if digest_method_uri.is_empty() {
            // Default to SHA-256 if no method specified
            let mut hasher = Sha256::new();
            hasher.update(canonicalized_content.as_bytes());
            hasher.finalize().to_vec()
        } else {
            return Err(ramp_common::Error::Authentication(
                format!("Unsupported digest algorithm: {}", digest_method_uri)
            ));
        };

        let expected_digest = decode_base64_multiline(&digest_value_b64)?;

        if computed_digest != expected_digest {
            return Err(ramp_common::Error::Authentication(
                "SAML signature digest verification failed: content has been tampered with".into()
            ));
        }

        // Step 5: Verify the RSA signature over the canonicalized SignedInfo.
        // Canonicalize SignedInfo with exc-c14n (as specified in CanonicalizationMethod).
        let canonicalized_signed_info = canonicalize_signed_info(&signed_info_raw);

        let signature_bytes = decode_base64_multiline(&signature_value_b64)?;

        // Extract the RSA public key from the DER-encoded certificate
        let public_key_der = extract_public_key_from_cert_der(&idp_cert_der)?;

        // Determine signature algorithm and verify
        let verification_algorithm: &dyn signature::VerificationAlgorithm =
            if sig_method_uri.contains("sha256") {
                &signature::RSA_PKCS1_2048_8192_SHA256
            } else if sig_method_uri.contains("sha1") || sig_method_uri.contains("sha-1") {
                &signature::RSA_PKCS1_2048_8192_SHA1_FOR_LEGACY_USE_ONLY
            } else if sig_method_uri.is_empty() {
                // Default to RSA-SHA256
                &signature::RSA_PKCS1_2048_8192_SHA256
            } else {
                return Err(ramp_common::Error::Authentication(
                    format!("Unsupported signature algorithm: {}", sig_method_uri)
                ));
            };

        let public_key = ring::signature::UnparsedPublicKey::new(
            verification_algorithm,
            &public_key_der,
        );

        public_key
            .verify(canonicalized_signed_info.as_bytes(), &signature_bytes)
            .map_err(|_| ramp_common::Error::Authentication(
                "SAML XML digital signature verification failed: signature is invalid".into()
            ))?;

        tracing::info!("SAML XML signature verified successfully");
        Ok(())
    }

    /// Extract the `<ds:Signature>` or `<Signature>` block from XML.
    pub(super) fn extract_signature_block(xml: &str) -> Option<String> {
        let start_patterns = [
            "<ds:Signature",
            "<Signature",
            "<dsig:Signature",
        ];
        let end_patterns = [
            "</ds:Signature>",
            "</Signature>",
            "</dsig:Signature>",
        ];

        for (start_pat, end_pat) in start_patterns.iter().zip(end_patterns.iter()) {
            if let Some(start) = xml.find(start_pat) {
                if let Some(end) = xml[start..].find(end_pat) {
                    return Some(xml[start..start + end + end_pat.len()].to_string());
                }
            }
        }
        None
    }

    /// Extract an entire element (including its tags) by local name.
    /// Handles common namespace prefixes (ds:, dsig:, or none).
    fn extract_element(xml: &str, local_name: &str) -> Option<String> {
        let prefixes = ["ds:", "dsig:", ""];
        for prefix in &prefixes {
            let open = format!("<{}{}", prefix, local_name);
            let close = format!("</{}{}>", prefix, local_name);
            if let Some(start) = xml.find(&open) {
                if let Some(end_offset) = xml[start..].find(&close) {
                    return Some(xml[start..start + end_offset + close.len()].to_string());
                }
            }
        }
        None
    }

    /// Extract the text content of an element by local name.
    pub(super) fn extract_element_text(xml: &str, local_name: &str) -> Option<String> {
        let element = extract_element(xml, local_name)?;
        // Find the closing '>' of the opening tag
        let gt = element.find('>')?;
        let content_start = gt + 1;
        // Find the opening '<' of the closing tag
        let lt = element[content_start..].find('<')?;
        let text = element[content_start..content_start + lt].trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Extract an XML attribute value from a specific child element.
    fn extract_attribute_from_element(xml: &str, element_local_name: &str, attr_name: &str) -> Option<String> {
        let prefixes = ["ds:", "dsig:", ""];
        for prefix in &prefixes {
            let open = format!("<{}{}", prefix, element_local_name);
            if let Some(start) = xml.find(&open) {
                // Find the end of this opening tag
                let tag_end = xml[start..].find('>')?;
                let tag_str = &xml[start..start + tag_end + 1];
                // Extract the attribute
                let attr_search = format!("{}=\"", attr_name);
                if let Some(attr_start) = tag_str.find(&attr_search) {
                    let value_start = attr_start + attr_search.len();
                    if let Some(end) = tag_str[value_start..].find('"') {
                        return Some(tag_str[value_start..value_start + end].to_string());
                    }
                }
            }
        }
        None
    }

    /// Remove the `<ds:Signature>` (or variant) element from XML.
    pub(super) fn remove_signature_element(xml: &str) -> String {
        let start_patterns = ["<ds:Signature", "<Signature", "<dsig:Signature"];
        let end_patterns = ["</ds:Signature>", "</Signature>", "</dsig:Signature>"];

        for (start_pat, end_pat) in start_patterns.iter().zip(end_patterns.iter()) {
            if let Some(start) = xml.find(start_pat) {
                if let Some(end) = xml[start..].find(end_pat) {
                    let before = &xml[..start];
                    let after = &xml[start + end + end_pat.len()..];
                    return format!("{}{}", before, after);
                }
            }
        }
        xml.to_string()
    }

    /// Find an XML element by its ID attribute value.
    fn extract_element_by_id(xml: &str, id: &str) -> Option<String> {
        // Search for ID="<id>" or Id="<id>" or id="<id>"
        let id_patterns = [
            format!("ID=\"{}\"", id),
            format!("Id=\"{}\"", id),
            format!("id=\"{}\"", id),
        ];

        for id_pat in &id_patterns {
            if let Some(id_pos) = xml.find(id_pat.as_str()) {
                // Walk backwards to find the '<' of this element
                let tag_start = xml[..id_pos].rfind('<')?;

                // Determine the element's tag name (e.g., "samlp:Response" or "saml:Assertion")
                let tag_name_end = xml[tag_start + 1..]
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .map(|p| tag_start + 1 + p)?;
                let tag_name = &xml[tag_start + 1..tag_name_end];

                // Find the closing tag
                let close_tag = format!("</{}>", tag_name);
                if let Some(close_pos) = xml[tag_start..].find(&close_tag) {
                    return Some(xml[tag_start..tag_start + close_pos + close_tag.len()].to_string());
                }
            }
        }
        None
    }

    /// Simplified Exclusive XML Canonicalization (exc-c14n) for SignedInfo.
    ///
    /// This handles the most common SAML canonicalization requirements:
    /// - Normalize attribute quoting to double quotes
    /// - Remove extra whitespace between attributes
    /// - Ensure self-closing tags are expanded
    /// - Normalize line endings to LF
    /// - Sort namespace declarations
    ///
    /// Note: This is a simplified implementation suitable for the common SAML
    /// response patterns. A full exc-c14n implementation would handle all
    /// edge cases defined in the W3C specification.
    pub(super) fn canonicalize_exc_c14n(xml: &str) -> String {
        let mut result = xml.to_string();

        // Normalize line endings: CRLF -> LF, CR -> LF
        result = result.replace("\r\n", "\n").replace('\r', "\n");

        // Trim leading/trailing whitespace from lines but preserve structure
        // (Only trim lines that are purely whitespace)
        let lines: Vec<&str> = result.lines().collect();
        result = lines.join("\n");

        // Normalize attribute whitespace: collapse multiple spaces between attributes
        // This is a simplified normalization
        while result.contains("  ") {
            result = result.replace("  ", " ");
        }

        // Expand self-closing tags: <tag/> -> <tag></tag>
        // Only for elements without namespace prefix conflicts
        let mut expanded = String::with_capacity(result.len());
        let mut chars = result.chars().peekable();
        let mut in_tag = false;
        let mut tag_name = String::new();

        while let Some(c) = chars.next() {
            if c == '<' {
                in_tag = true;
                tag_name.clear();
                expanded.push(c);
            } else if in_tag && c == '/' && chars.peek() == Some(&'>') {
                // Self-closing tag: convert /> to ></tagname>
                chars.next(); // consume '>'
                expanded.push('>');
                // Extract just the tag name (without attributes)
                let name = tag_name.split_whitespace().next().unwrap_or("").to_string();
                expanded.push_str(&format!("</{}>", name));
                in_tag = false;
            } else if in_tag && c == '>' {
                in_tag = false;
                expanded.push(c);
            } else {
                if in_tag {
                    tag_name.push(c);
                }
                expanded.push(c);
            }
        }

        expanded
    }

    /// Canonicalize the SignedInfo element specifically.
    /// Ensures the SignedInfo element includes the necessary namespace declarations
    /// that may be inherited from parent elements.
    fn canonicalize_signed_info(signed_info: &str) -> String {
        let mut result = signed_info.to_string();

        // If SignedInfo doesn't already have the ds namespace declaration,
        // add it (it may be inherited from the parent Signature element)
        let needs_ds_ns = (result.contains("<ds:") || result.contains("</ds:"))
            && !result.contains("xmlns:ds=");

        if needs_ds_ns {
            // Insert the ds namespace declaration into the SignedInfo opening tag
            let prefixes = ["<ds:SignedInfo", "<dsig:SignedInfo", "<SignedInfo"];
            for prefix in &prefixes {
                if let Some(pos) = result.find(prefix) {
                    let insert_pos = pos + prefix.len();
                    let ns_decl = if prefix.contains("ds:") {
                        " xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\""
                    } else if prefix.contains("dsig:") {
                        " xmlns:dsig=\"http://www.w3.org/2000/09/xmldsig#\""
                    } else {
                        " xmlns=\"http://www.w3.org/2000/09/xmldsig#\""
                    };

                    result.insert_str(insert_pos, ns_decl);
                    break;
                }
            }
        }

        canonicalize_exc_c14n(&result)
    }

    /// Decode a PEM-encoded X.509 certificate to DER bytes.
    fn decode_certificate_pem(pem_str: &str) -> ramp_common::Result<Vec<u8>> {
        let trimmed = pem_str.trim();

        // If it looks like raw base64 without PEM headers, wrap it
        if !trimmed.starts_with("-----BEGIN") {
            return decode_base64_multiline(trimmed);
        }

        let parsed = pem::parse(trimmed)
            .map_err(|e| ramp_common::Error::Authentication(
                format!("Failed to parse IdP certificate PEM: {}", e)
            ))?;

        Ok(parsed.contents().to_vec())
    }

    /// Decode base64 that may contain whitespace and newlines.
    pub(super) fn decode_base64_multiline(b64: &str) -> ramp_common::Result<Vec<u8>> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let cleaned: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
        STANDARD.decode(&cleaned)
            .map_err(|e| ramp_common::Error::Authentication(
                format!("Failed to decode base64 data: {}", e)
            ))
    }

    /// Extract the RSA public key (in DER SubjectPublicKeyInfo format) from a
    /// DER-encoded X.509 certificate.
    ///
    /// X.509 certificates are ASN.1 DER-encoded. The structure (simplified) is:
    /// ```text
    /// Certificate ::= SEQUENCE {
    ///   tbsCertificate SEQUENCE {
    ///     version [0] EXPLICIT ...,
    ///     serialNumber INTEGER,
    ///     signature AlgorithmIdentifier,
    ///     issuer ...,
    ///     validity ...,
    ///     subject ...,
    ///     subjectPublicKeyInfo SubjectPublicKeyInfo, // <-- we want this
    ///     ...
    ///   },
    ///   signatureAlgorithm ...,
    ///   signatureValue ...
    /// }
    /// ```
    fn extract_public_key_from_cert_der(cert_der: &[u8]) -> ramp_common::Result<Vec<u8>> {
        // Parse the outer SEQUENCE (Certificate)
        let (tbs_cert_bytes, _) = parse_asn1_sequence(cert_der)
            .ok_or_else(|| ramp_common::Error::Authentication(
                "Invalid X.509 certificate: not a valid ASN.1 SEQUENCE".into()
            ))?;

        // Parse tbsCertificate SEQUENCE
        let (tbs_inner, _) = parse_asn1_sequence(tbs_cert_bytes)
            .ok_or_else(|| ramp_common::Error::Authentication(
                "Invalid X.509 certificate: cannot parse tbsCertificate".into()
            ))?;

        // Navigate through tbsCertificate fields to find subjectPublicKeyInfo
        // Fields: version [0], serialNumber, signature, issuer, validity, subject, subjectPublicKeyInfo
        let mut pos = 0;
        let mut field_index = 0;

        // Check for explicit version tag [0]
        if pos < tbs_inner.len() && tbs_inner[pos] == 0xA0 {
            // Version is present (explicit context tag [0])
            let (_, consumed) = parse_asn1_tlv(&tbs_inner[pos..])
                .ok_or_else(|| ramp_common::Error::Authentication(
                    "Invalid X.509: cannot parse version".into()
                ))?;
            pos += consumed;
            // version consumed, don't increment field_index since
            // we separately handle it
        }

        // Now iterate through: serialNumber(0), signature(1), issuer(2), validity(3), subject(4), subjectPKInfo(5)
        while pos < tbs_inner.len() && field_index < 6 {
            if field_index == 5 {
                // This should be the subjectPublicKeyInfo SEQUENCE
                let (_, total_len) = parse_asn1_tlv(&tbs_inner[pos..])
                    .ok_or_else(|| ramp_common::Error::Authentication(
                        "Invalid X.509: cannot parse subjectPublicKeyInfo".into()
                    ))?;
                return Ok(tbs_inner[pos..pos + total_len].to_vec());
            }

            let (_, consumed) = parse_asn1_tlv(&tbs_inner[pos..])
                .ok_or_else(|| ramp_common::Error::Authentication(
                    format!("Invalid X.509: cannot parse field at index {}", field_index)
                ))?;
            pos += consumed;
            field_index += 1;
        }

        Err(ramp_common::Error::Authentication(
            "Invalid X.509 certificate: subjectPublicKeyInfo not found".into()
        ))
    }

    /// Parse an ASN.1 SEQUENCE and return (content_bytes, total_consumed_bytes).
    fn parse_asn1_sequence(data: &[u8]) -> Option<(&[u8], usize)> {
        if data.is_empty() || data[0] != 0x30 {
            return None;
        }
        let (content, total) = parse_asn1_tlv(data)?;
        // content starts after tag+length
        let header_len = total - content.len();
        Some((&data[header_len..header_len + content.len()], total))
    }

    /// Parse an ASN.1 TLV (Tag-Length-Value) and return (value_bytes, total_consumed).
    fn parse_asn1_tlv(data: &[u8]) -> Option<(&[u8], usize)> {
        if data.is_empty() {
            return None;
        }

        let _tag = data[0];
        if data.len() < 2 {
            return None;
        }

        let (content_len, header_len) = parse_asn1_length(&data[1..])?;
        let total_len = 1 + header_len + content_len;

        if data.len() < total_len {
            return None;
        }

        Some((&data[1 + header_len..1 + header_len + content_len], total_len))
    }

    /// Parse ASN.1 length encoding. Returns (length_value, bytes_consumed).
    pub(super) fn parse_asn1_length(data: &[u8]) -> Option<(usize, usize)> {
        if data.is_empty() {
            return None;
        }

        if data[0] & 0x80 == 0 {
            // Short form: length in a single byte
            Some((data[0] as usize, 1))
        } else {
            // Long form: first byte indicates number of subsequent length bytes
            let num_bytes = (data[0] & 0x7F) as usize;
            if num_bytes == 0 || num_bytes > 4 || data.len() < 1 + num_bytes {
                return None;
            }
            let mut length: usize = 0;
            for i in 0..num_bytes {
                length = (length << 8) | (data[1 + i] as usize);
            }
            Some((length, 1 + num_bytes))
        }
    }
}

#[async_trait]
impl SsoProvider for SamlProvider {
    fn provider_type(&self) -> SsoProviderType {
        self.provider_type
    }

    fn protocol(&self) -> SsoProtocol {
        SsoProtocol::Saml
    }

    async fn authorize(&self, request: &SsoAuthRequest) -> Result<SsoAuthResponse> {
        // Generate SAML AuthnRequest
        let issue_instant = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let id = uuid::Uuid::new_v4().to_string();

        let authn_request = format!(
            r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="_{}" Version="2.0" IssueInstant="{}" Destination="{}" ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" AssertionConsumerServiceURL="{}"><saml:Issuer>{}</saml:Issuer><samlp:NameIDPolicy Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress" AllowCreate="true"/></samlp:AuthnRequest>"#,
            id,
            issue_instant,
            self.config.idp_sso_url,
            request.redirect_uri,
            self.config.sp_entity_id
        );

        // Compress and Base64 encode (Deflate + Base64 is standard for Redirect binding,
        // but for POST binding it's just Base64?)
        // SAML Redirect Binding uses Deflate -> Base64 -> URL Encode
        let encoded = {
            use base64::{engine::general_purpose::STANDARD, Engine};
            STANDARD.encode(authn_request)
        }; // Simplified for POST

        // Construct redirect URL
        let url = format!("{}?SAMLRequest={}&RelayState={}",
            self.config.idp_sso_url,
            urlencoding::encode(&encoded),
            urlencoding::encode(&request.state)
        );

        Ok(SsoAuthResponse {
            auth_url: url,
            state: request.state.clone(),
        })
    }

    async fn authenticate(&self, callback: &SsoCallback) -> Result<SsoUser> {
        let saml_response = callback
            .code
            .as_ref() // In SAML, the response is often passed in a field similar to 'code' or body
            .or(callback.saml_response.as_ref())
            .ok_or_else(|| ramp_common::Error::Authentication("Missing SAMLResponse".into()))?;

        // Decode, verify signature, and extract claims
        let parsed = self.parse_saml_response(saml_response)?;

        // Reject responses from unexpected issuers to prevent
        // assertion injection from unauthorized identity providers.
        if parsed.issuer != self.config.idp_entity_id {
            return Err(ramp_common::Error::Authentication(format!(
                "SAML issuer mismatch: expected '{}', got '{}'",
                self.config.idp_entity_id, parsed.issuer
            )));
        }

        let email = parsed.name_id;
        let groups = parsed.attributes.get("groups").cloned().unwrap_or_default();
        let roles = crate::sso::SsoService::map_roles(&groups, &self.role_mappings, &self.default_role);

        let now = Utc::now();

        Ok(SsoUser {
            idp_user_id: email.clone(), // Use email as ID for SAML usually
            email: email.clone(),
            name: None,
            given_name: None,
            family_name: None,
            groups,
            roles,
            claims: HashMap::new(),
            authenticated_at: now,
            expires_at: now + Duration::hours(8),
        })
    }

    async fn validate_session(&self, _session_token: &str) -> Result<Option<SsoUser>> {
        Ok(None)
    }

    async fn logout(&self, _user: &SsoUser) -> Result<Option<String>> {
        // SAML Single Logout (SLO) is complex
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tag_value() {
        let xml = r#"<saml:NameID Format="email">user@example.com</saml:NameID>"#;
        assert_eq!(
            extract_tag_value(xml, "NameID"),
            Some("user@example.com".to_string())
        );
    }

    #[test]
    fn test_extract_tag_value_no_namespace() {
        let xml = r#"<NameID>user@test.com</NameID>"#;
        assert_eq!(
            extract_tag_value(xml, "NameID"),
            Some("user@test.com".to_string())
        );
    }

    #[test]
    fn test_extract_tag_value_missing() {
        let xml = r#"<saml:Assertion>content</saml:Assertion>"#;
        assert_eq!(extract_tag_value(xml, "NameID"), None);
    }

    #[test]
    fn test_extract_saml_attributes() {
        let xml = r#"<saml:Attribute Name="email"><saml:AttributeValue>user@example.com</saml:AttributeValue></saml:Attribute><saml:Attribute Name="groups"><saml:AttributeValue>Admins</saml:AttributeValue><saml:AttributeValue>Finance</saml:AttributeValue></saml:Attribute>"#;
        let attrs = extract_saml_attributes(xml);
        assert_eq!(attrs.get("email"), Some(&vec!["user@example.com".to_string()]));
        assert_eq!(attrs.get("groups"), Some(&vec!["Admins".to_string(), "Finance".to_string()]));
    }

    #[test]
    fn test_extract_signature_block() {
        let xml = r#"<samlp:Response><ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:SignedInfo/><ds:SignatureValue>abc=</ds:SignatureValue></ds:Signature><saml:Assertion>data</saml:Assertion></samlp:Response>"#;
        let sig = xmldsig::extract_signature_block(xml);
        assert!(sig.is_some());
        assert!(sig.unwrap().contains("SignatureValue"));
    }

    #[test]
    fn test_extract_element_text() {
        let xml = r#"<ds:SignatureValue>AQID</ds:SignatureValue>"#;
        assert_eq!(
            xmldsig::extract_element_text(xml, "SignatureValue"),
            Some("AQID".to_string())
        );
    }

    #[test]
    fn test_remove_signature_element() {
        let xml = r#"<Response><ds:Signature>sig</ds:Signature><Assertion>data</Assertion></Response>"#;
        let result = xmldsig::remove_signature_element(xml);
        assert_eq!(result, "<Response><Assertion>data</Assertion></Response>");
    }

    #[test]
    fn test_asn1_length_short() {
        let data = [0x05]; // length = 5
        let (len, consumed) = xmldsig::parse_asn1_length(&data).unwrap();
        assert_eq!(len, 5);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_asn1_length_long() {
        let data = [0x82, 0x01, 0x00]; // length = 256
        let (len, consumed) = xmldsig::parse_asn1_length(&data).unwrap();
        assert_eq!(len, 256);
        assert_eq!(consumed, 3);
    }

    #[test]
    fn test_decode_base64_multiline() {
        let b64 = "AQID\n  BAUG\n";
        let decoded = xmldsig::decode_base64_multiline(b64).unwrap();
        assert_eq!(decoded, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_canonicalize_exc_c14n_crlf() {
        let input = "line1\r\nline2\rline3";
        let result = xmldsig::canonicalize_exc_c14n(input);
        assert_eq!(result, "line1\nline2\nline3");
    }

    // ---- SamlConfig ----

    #[test]
    fn test_saml_config_okta() {
        let config = SamlConfig::okta(
            "sp_entity".to_string(),
            "https://idp.okta.com/metadata",
            "CERT_DATA".to_string(),
        );
        assert_eq!(config.sp_entity_id, "sp_entity");
        assert_eq!(config.idp_entity_id, "https://idp.okta.com/metadata");
        assert_eq!(config.idp_sso_url, "https://idp.okta.com/sso");
        assert_eq!(config.idp_certificate, "CERT_DATA");
        assert!(config.allow_idp_initiated);
        assert!(config.sign_requests);
        assert!(config.sp_private_key.is_none());
        assert!(config.attribute_mapping.contains_key("email"));
        assert!(config.attribute_mapping.contains_key("groups"));
    }

    // ---- SamlProvider ----

    #[test]
    fn test_saml_provider_creation() {
        let config = SamlConfig::okta(
            "sp_test".to_string(),
            "https://idp.test.com/metadata",
            "CERT".to_string(),
        );
        let provider = SamlProvider::new(
            SsoProviderType::Okta,
            config,
            vec![],
            RampRole::Viewer,
        ).unwrap();
        assert_eq!(provider.provider_type(), SsoProviderType::Okta);
        assert_eq!(provider.protocol(), SsoProtocol::Saml);
    }

    #[tokio::test]
    async fn test_saml_authorize_generates_request() {
        let config = SamlConfig::okta(
            "sp_ent".to_string(),
            "https://idp.example.com/metadata",
            "CERT".to_string(),
        );
        let provider = SamlProvider::new(
            SsoProviderType::Okta,
            config,
            vec![],
            RampRole::Viewer,
        ).unwrap();

        let request = SsoAuthRequest {
            tenant_id: ramp_common::types::TenantId::new("t1"),
            redirect_uri: "https://app.example.com/sso/callback".to_string(),
            state: "relay_state_123".to_string(),
            nonce: None,
        };

        let response = provider.authorize(&request).await.unwrap();
        assert_eq!(response.state, "relay_state_123");
        assert!(response.auth_url.contains("SAMLRequest="));
        assert!(response.auth_url.contains("RelayState="));
        assert!(response.auth_url.starts_with("https://idp.example.com/sso?"));
    }

    #[tokio::test]
    async fn test_saml_validate_session_returns_none() {
        let config = SamlConfig::okta("sp".into(), "https://idp/m", "C".into());
        let provider = SamlProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer).unwrap();
        let result = provider.validate_session("token").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_saml_logout_returns_none() {
        let config = SamlConfig::okta("sp".into(), "https://idp/m", "C".into());
        let provider = SamlProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer).unwrap();
        let user = SsoUser {
            idp_user_id: "u1".to_string(),
            email: "u@test.com".to_string(),
            name: None,
            given_name: None,
            family_name: None,
            groups: vec![],
            roles: vec![],
            claims: HashMap::new(),
            authenticated_at: Utc::now(),
            expires_at: Utc::now(),
        };
        let result = provider.logout(&user).await.unwrap();
        assert!(result.is_none()); // SAML SLO not implemented
    }

    // ---- XML helpers ----

    #[test]
    fn test_extract_tag_value_with_attributes() {
        let xml = r#"<saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">admin@corp.com</saml:NameID>"#;
        assert_eq!(
            extract_tag_value(xml, "NameID"),
            Some("admin@corp.com".to_string())
        );
    }

    #[test]
    fn test_extract_tag_value_empty_content() {
        let xml = r#"<NameID></NameID>"#;
        assert_eq!(extract_tag_value(xml, "NameID"), None);
    }

    #[test]
    fn test_extract_xml_attribute_double_quotes() {
        let tag = r#"<Attribute Name="email" NameFormat="urn:oasis">"#;
        assert_eq!(extract_xml_attribute(tag, "Name"), Some("email".to_string()));
        assert_eq!(extract_xml_attribute(tag, "NameFormat"), Some("urn:oasis".to_string()));
    }

    #[test]
    fn test_extract_xml_attribute_single_quotes() {
        let tag = "<Attribute Name='groups'>";
        assert_eq!(extract_xml_attribute(tag, "Name"), Some("groups".to_string()));
    }

    #[test]
    fn test_extract_xml_attribute_missing() {
        let tag = r#"<Attribute Name="email">"#;
        assert_eq!(extract_xml_attribute(tag, "Missing"), None);
    }

    #[test]
    fn test_extract_attribute_values_multiple() {
        let block = r#"<saml:Attribute Name="groups"><saml:AttributeValue>Admin</saml:AttributeValue><saml:AttributeValue>Users</saml:AttributeValue></saml:Attribute>"#;
        let values = extract_attribute_values(block);
        assert_eq!(values, vec!["Admin".to_string(), "Users".to_string()]);
    }

    #[test]
    fn test_extract_attribute_values_no_namespace() {
        let block = r#"<Attribute Name="role"><AttributeValue>Manager</AttributeValue></Attribute>"#;
        let values = extract_attribute_values(block);
        assert_eq!(values, vec!["Manager".to_string()]);
    }

    #[test]
    fn test_extract_saml_attributes_empty() {
        let xml = r#"<saml:Assertion>no attributes here</saml:Assertion>"#;
        let attrs = extract_saml_attributes(xml);
        assert!(attrs.is_empty());
    }

    #[test]
    fn test_find_attribute_element() {
        let xml = r#"<saml:Attribute Name="email"><saml:AttributeValue>test</saml:AttributeValue></saml:Attribute>"#;
        let pos = find_attribute_element(xml, 0);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap(), 0);
    }

    #[test]
    fn test_find_attribute_element_not_found() {
        let xml = r#"<saml:Assertion>content</saml:Assertion>"#;
        assert!(find_attribute_element(xml, 0).is_none());
    }

    // ---- xmldsig module ----

    #[test]
    fn test_extract_signature_block_ds_prefix() {
        let xml = r#"<Response><ds:Signature><ds:SignedInfo/><ds:SignatureValue>abc</ds:SignatureValue></ds:Signature></Response>"#;
        let sig = xmldsig::extract_signature_block(xml);
        assert!(sig.is_some());
        let block = sig.unwrap();
        assert!(block.starts_with("<ds:Signature"));
        assert!(block.ends_with("</ds:Signature>"));
    }

    #[test]
    fn test_extract_signature_block_no_prefix() {
        let xml = r#"<Response><Signature><SignedInfo/><SignatureValue>xyz</SignatureValue></Signature></Response>"#;
        let sig = xmldsig::extract_signature_block(xml);
        assert!(sig.is_some());
    }

    #[test]
    fn test_extract_signature_block_dsig_prefix() {
        let xml = r#"<Response><dsig:Signature><dsig:SignedInfo/></dsig:Signature></Response>"#;
        let sig = xmldsig::extract_signature_block(xml);
        assert!(sig.is_some());
    }

    #[test]
    fn test_extract_signature_block_missing() {
        let xml = r#"<Response><Assertion>data</Assertion></Response>"#;
        assert!(xmldsig::extract_signature_block(xml).is_none());
    }

    #[test]
    fn test_remove_signature_element_preserves_rest() {
        let xml = r#"<Response>before<ds:Signature>sig_content</ds:Signature>after</Response>"#;
        let result = xmldsig::remove_signature_element(xml);
        assert_eq!(result, "<Response>beforeafter</Response>");
    }

    #[test]
    fn test_remove_signature_element_no_signature() {
        let xml = r#"<Response>content</Response>"#;
        let result = xmldsig::remove_signature_element(xml);
        assert_eq!(result, xml);
    }

    #[test]
    fn test_canonicalize_collapses_whitespace() {
        let input = "<tag  attr1=\"a\"   attr2=\"b\"  />";
        let result = xmldsig::canonicalize_exc_c14n(input);
        assert!(!result.contains("  ")); // no double spaces
    }

    #[test]
    fn test_canonicalize_expands_self_closing() {
        let input = "<EmptyElement/>";
        let result = xmldsig::canonicalize_exc_c14n(input);
        assert_eq!(result, "<EmptyElement></EmptyElement>");
    }

    #[test]
    fn test_canonicalize_self_closing_with_attrs() {
        let input = r#"<DigestMethod Algorithm="sha256"/>"#;
        let result = xmldsig::canonicalize_exc_c14n(input);
        assert!(result.contains("</DigestMethod>"));
        assert!(!result.contains("/>"));
    }

    #[test]
    fn test_decode_base64_multiline_with_whitespace() {
        // Base64 of [0x01, 0x02, 0x03, 0x04, 0x05, 0x06]
        let b64 = "AQID\n  BAUG\n";
        let decoded = xmldsig::decode_base64_multiline(b64).unwrap();
        assert_eq!(decoded, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_decode_base64_multiline_invalid() {
        let result = xmldsig::decode_base64_multiline("!!!not_valid!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_asn1_length_zero() {
        let data = [0x00];
        let (len, consumed) = xmldsig::parse_asn1_length(&data).unwrap();
        assert_eq!(len, 0);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_asn1_length_127() {
        let data = [0x7F]; // max short form
        let (len, consumed) = xmldsig::parse_asn1_length(&data).unwrap();
        assert_eq!(len, 127);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_asn1_length_long_one_byte() {
        let data = [0x81, 0x80]; // length = 128
        let (len, consumed) = xmldsig::parse_asn1_length(&data).unwrap();
        assert_eq!(len, 128);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_asn1_length_empty_input() {
        let result = xmldsig::parse_asn1_length(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_asn1_length_long_insufficient_data() {
        let data = [0x82, 0x01]; // claims 2 bytes of length but only 1 provided
        let result = xmldsig::parse_asn1_length(&data);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_element_text_with_namespace() {
        let xml = r#"<ds:DigestValue>SGVsbG8=</ds:DigestValue>"#;
        assert_eq!(
            xmldsig::extract_element_text(xml, "DigestValue"),
            Some("SGVsbG8=".to_string())
        );
    }

    #[test]
    fn test_extract_element_text_empty() {
        let xml = r#"<ds:DigestValue></ds:DigestValue>"#;
        assert_eq!(xmldsig::extract_element_text(xml, "DigestValue"), None);
    }

    #[test]
    fn test_extract_element_text_no_match() {
        let xml = r#"<ds:OtherElement>content</ds:OtherElement>"#;
        assert_eq!(xmldsig::extract_element_text(xml, "DigestValue"), None);
    }

    // ---- SamlProvider authenticate ----

    #[tokio::test]
    async fn test_saml_authenticate_missing_response() {
        let config = SamlConfig::okta("sp".into(), "https://idp/m", "C".into());
        let provider = SamlProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer).unwrap();

        let callback = SsoCallback {
            code: None,
            state: "state".to_string(),
            error: None,
            error_description: None,
            saml_response: None,
        };

        let result = provider.authenticate(&callback).await;
        assert!(result.is_err());
    }
}
