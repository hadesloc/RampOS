# SSO Setup Guide

This guide covers Single Sign-On (SSO) configuration for RampOS Enterprise using SAML 2.0 and OpenID Connect (OIDC).

---

## Overview

RampOS supports two SSO protocols:

| Protocol | Best For | Features |
|----------|----------|----------|
| **OIDC** | Modern IdPs, cloud-native | Simpler setup, JWT tokens, refresh tokens |
| **SAML 2.0** | Enterprise IdPs, legacy systems | Mature protocol, wide compatibility |

---

## Supported Identity Providers

| Provider | OIDC | SAML | Documentation |
|----------|------|------|---------------|
| Okta | Yes | Yes | [Okta Setup](#okta) |
| Azure AD | Yes | Yes | [Azure AD Setup](#azure-ad) |
| Google Workspace | Yes | Yes | [Google Setup](#google-workspace) |
| Auth0 | Yes | Yes | [Auth0 Setup](#auth0) |
| Ping Identity | Yes | Yes | Contact support |
| OneLogin | Yes | Yes | Contact support |
| Custom OIDC | Yes | - | [Custom OIDC](#custom-oidc-provider) |
| Custom SAML | - | Yes | [Custom SAML](#custom-saml-provider) |

---

## OIDC Configuration

### Environment Variables

```bash
# Enable SSO
SSO_ENABLED=true
SSO_PROVIDER=oidc

# OIDC Configuration
OIDC_ISSUER_URL=https://your-idp.com
OIDC_CLIENT_ID=your_client_id
OIDC_CLIENT_SECRET=your_client_secret
OIDC_REDIRECT_URI=https://api.your-domain.com/auth/callback

# Optional
OIDC_SCOPES=openid,profile,email,groups
OIDC_AUDIENCE=rampos-api
```

### Kubernetes Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: rampos-sso
  namespace: rampos
type: Opaque
stringData:
  SSO_ENABLED: "true"
  SSO_PROVIDER: "oidc"
  OIDC_ISSUER_URL: "https://your-idp.com"
  OIDC_CLIENT_ID: "your_client_id"
  OIDC_CLIENT_SECRET: "your_client_secret"
  OIDC_REDIRECT_URI: "https://api.your-domain.com/auth/callback"
```

---

## Okta

### OIDC Setup

#### 1. Create Application in Okta

1. Log in to Okta Admin Console
2. Navigate to **Applications** > **Create App Integration**
3. Select **OIDC - OpenID Connect**
4. Select **Web Application**
5. Configure:
   - **App integration name**: RampOS
   - **Grant type**: Authorization Code, Refresh Token
   - **Sign-in redirect URIs**: `https://api.your-domain.com/auth/callback`
   - **Sign-out redirect URIs**: `https://your-domain.com/logout`

#### 2. Configure Okta

1. In the application settings, note:
   - **Client ID**
   - **Client Secret**
   - **Okta Domain** (e.g., `your-org.okta.com`)

2. Under **Assignments**, add users/groups who should access RampOS

#### 3. Configure RampOS

```bash
SSO_ENABLED=true
SSO_PROVIDER=oidc
OIDC_ISSUER_URL=https://your-org.okta.com
OIDC_CLIENT_ID=0oaxxxxxxxxxxxxxxxx
OIDC_CLIENT_SECRET=your_client_secret
OIDC_REDIRECT_URI=https://api.your-domain.com/auth/callback
OIDC_SCOPES=openid,profile,email,groups
```

### SAML Setup

#### 1. Create SAML Application in Okta

1. Navigate to **Applications** > **Create App Integration**
2. Select **SAML 2.0**
3. Configure:
   - **App name**: RampOS
   - **Single sign on URL**: `https://api.your-domain.com/auth/saml/acs`
   - **Audience URI (SP Entity ID)**: `https://api.your-domain.com/auth/saml/metadata`
   - **Name ID format**: EmailAddress
   - **Application username**: Email

4. Configure attribute statements:

| Name | Value |
|------|-------|
| email | user.email |
| firstName | user.firstName |
| lastName | user.lastName |
| groups | user.groups |

#### 2. Download Metadata

1. In the application, go to **Sign On** tab
2. Click **Identity Provider metadata** to download

#### 3. Configure RampOS

```bash
SSO_ENABLED=true
SSO_PROVIDER=saml
SAML_IDP_METADATA_URL=https://your-org.okta.com/app/exkxxxxxxxx/sso/saml/metadata
SAML_SP_ENTITY_ID=https://api.your-domain.com/auth/saml/metadata
SAML_SP_ACS_URL=https://api.your-domain.com/auth/saml/acs
SAML_SP_SLO_URL=https://api.your-domain.com/auth/saml/slo
SAML_ATTRIBUTE_EMAIL=email
SAML_ATTRIBUTE_FIRST_NAME=firstName
SAML_ATTRIBUTE_LAST_NAME=lastName
SAML_ATTRIBUTE_GROUPS=groups
```

---

## Azure AD

### OIDC Setup

#### 1. Register Application in Azure

1. Go to Azure Portal > **Azure Active Directory** > **App registrations**
2. Click **New registration**
3. Configure:
   - **Name**: RampOS
   - **Supported account types**: Choose based on requirements
   - **Redirect URI**: Web, `https://api.your-domain.com/auth/callback`

#### 2. Configure Application

1. Under **Certificates & secrets**, create a new client secret
2. Note the **Application (client) ID** and **Directory (tenant) ID**
3. Under **API permissions**, add:
   - `openid`
   - `profile`
   - `email`
   - `User.Read`
   - `GroupMember.Read.All` (for groups)

4. Grant admin consent for the permissions

#### 3. Configure RampOS

```bash
SSO_ENABLED=true
SSO_PROVIDER=oidc
OIDC_ISSUER_URL=https://login.microsoftonline.com/YOUR_TENANT_ID/v2.0
OIDC_CLIENT_ID=your_application_id
OIDC_CLIENT_SECRET=your_client_secret
OIDC_REDIRECT_URI=https://api.your-domain.com/auth/callback
OIDC_SCOPES=openid,profile,email
```

### SAML Setup

#### 1. Create Enterprise Application

1. Go to **Azure AD** > **Enterprise applications** > **New application**
2. Click **Create your own application**
3. Select **Integrate any other application you don't find in the gallery**

#### 2. Configure SAML

1. Under **Single sign-on**, select **SAML**
2. Configure **Basic SAML Configuration**:
   - **Identifier (Entity ID)**: `https://api.your-domain.com/auth/saml/metadata`
   - **Reply URL (ACS URL)**: `https://api.your-domain.com/auth/saml/acs`
   - **Sign on URL**: `https://your-domain.com/login`

3. Configure **Attributes & Claims**:
   - email: `user.mail`
   - firstName: `user.givenname`
   - lastName: `user.surname`
   - groups: `user.groups`

4. Download **Federation Metadata XML**

#### 3. Configure RampOS

```bash
SSO_ENABLED=true
SSO_PROVIDER=saml
SAML_IDP_METADATA_URL=https://login.microsoftonline.com/YOUR_TENANT_ID/federationmetadata/2007-06/federationmetadata.xml
SAML_SP_ENTITY_ID=https://api.your-domain.com/auth/saml/metadata
SAML_SP_ACS_URL=https://api.your-domain.com/auth/saml/acs
```

---

## Google Workspace

### OIDC Setup

#### 1. Create OAuth Client

1. Go to Google Cloud Console > **APIs & Services** > **Credentials**
2. Click **Create Credentials** > **OAuth client ID**
3. Configure:
   - **Application type**: Web application
   - **Authorized redirect URIs**: `https://api.your-domain.com/auth/callback`

#### 2. Configure RampOS

```bash
SSO_ENABLED=true
SSO_PROVIDER=oidc
OIDC_ISSUER_URL=https://accounts.google.com
OIDC_CLIENT_ID=your_client_id.apps.googleusercontent.com
OIDC_CLIENT_SECRET=your_client_secret
OIDC_REDIRECT_URI=https://api.your-domain.com/auth/callback
OIDC_SCOPES=openid,profile,email
```

### SAML Setup

#### 1. Configure SAML App in Google Admin

1. Go to **Google Admin Console** > **Apps** > **Web and mobile apps**
2. Click **Add app** > **Add custom SAML app**
3. Configure:
   - **App name**: RampOS
   - **ACS URL**: `https://api.your-domain.com/auth/saml/acs`
   - **Entity ID**: `https://api.your-domain.com/auth/saml/metadata`
   - **Name ID format**: EMAIL
   - **Name ID**: Basic Information > Primary email

4. Add attribute mappings:
   - email: Primary email
   - firstName: First name
   - lastName: Last name

5. Download IdP metadata

#### 2. Configure RampOS

```bash
SSO_ENABLED=true
SSO_PROVIDER=saml
SAML_IDP_METADATA_FILE=/path/to/google-idp-metadata.xml
SAML_SP_ENTITY_ID=https://api.your-domain.com/auth/saml/metadata
SAML_SP_ACS_URL=https://api.your-domain.com/auth/saml/acs
```

---

## Auth0

### OIDC Setup

#### 1. Create Application

1. Go to Auth0 Dashboard > **Applications** > **Create Application**
2. Select **Regular Web Applications**
3. Configure:
   - **Allowed Callback URLs**: `https://api.your-domain.com/auth/callback`
   - **Allowed Logout URLs**: `https://your-domain.com`

#### 2. Configure RampOS

```bash
SSO_ENABLED=true
SSO_PROVIDER=oidc
OIDC_ISSUER_URL=https://your-tenant.auth0.com
OIDC_CLIENT_ID=your_client_id
OIDC_CLIENT_SECRET=your_client_secret
OIDC_REDIRECT_URI=https://api.your-domain.com/auth/callback
OIDC_SCOPES=openid,profile,email
```

---

## Custom OIDC Provider

### Requirements

Your OIDC provider must support:
- Authorization Code flow
- Discovery endpoint (`/.well-known/openid-configuration`)
- Standard claims: `sub`, `email`, `name`

### Configuration

```bash
SSO_ENABLED=true
SSO_PROVIDER=oidc

# Provider configuration
OIDC_ISSUER_URL=https://your-idp.com
OIDC_CLIENT_ID=your_client_id
OIDC_CLIENT_SECRET=your_client_secret
OIDC_REDIRECT_URI=https://api.your-domain.com/auth/callback

# Optional: Override discovery
OIDC_AUTHORIZATION_ENDPOINT=https://your-idp.com/oauth/authorize
OIDC_TOKEN_ENDPOINT=https://your-idp.com/oauth/token
OIDC_USERINFO_ENDPOINT=https://your-idp.com/oauth/userinfo
OIDC_JWKS_URI=https://your-idp.com/.well-known/jwks.json

# Scopes
OIDC_SCOPES=openid,profile,email

# Claim mappings
OIDC_CLAIM_EMAIL=email
OIDC_CLAIM_NAME=name
OIDC_CLAIM_GROUPS=groups
```

---

## Custom SAML Provider

### Requirements

Your SAML IdP must support:
- SAML 2.0
- HTTP-POST binding
- Signed assertions

### Configuration

```bash
SSO_ENABLED=true
SSO_PROVIDER=saml

# IdP Configuration (choose one)
SAML_IDP_METADATA_URL=https://your-idp.com/saml/metadata
# OR
SAML_IDP_METADATA_FILE=/path/to/idp-metadata.xml

# SP Configuration
SAML_SP_ENTITY_ID=https://api.your-domain.com/auth/saml/metadata
SAML_SP_ACS_URL=https://api.your-domain.com/auth/saml/acs
SAML_SP_SLO_URL=https://api.your-domain.com/auth/saml/slo

# Signing (optional)
SAML_SP_PRIVATE_KEY_FILE=/path/to/sp-private.pem
SAML_SP_CERTIFICATE_FILE=/path/to/sp-certificate.pem

# Attribute mappings
SAML_ATTRIBUTE_EMAIL=urn:oid:0.9.2342.19200300.100.1.3
SAML_ATTRIBUTE_FIRST_NAME=urn:oid:2.5.4.42
SAML_ATTRIBUTE_LAST_NAME=urn:oid:2.5.4.4
SAML_ATTRIBUTE_GROUPS=memberOf
```

---

## Role Mapping

### Map IdP Groups to RampOS Roles

Configure role mappings to automatically assign permissions based on IdP group membership:

```bash
# Role mapping configuration
SSO_ROLE_MAPPING_ENABLED=true
SSO_DEFAULT_ROLE=viewer
SSO_ADMIN_GROUPS=rampos-admins,platform-admins
SSO_OPERATOR_GROUPS=rampos-operators,compliance-team
SSO_VIEWER_GROUPS=rampos-viewers
```

### Role Mapping via API

```bash
# Configure role mappings
curl -X PUT https://api.your-domain.com/v1/admin/sso/role-mappings \
  -H "Authorization: Bearer admin_token" \
  -H "Content-Type: application/json" \
  -d '{
    "mappings": [
      {
        "idpGroup": "rampos-admins",
        "role": "admin"
      },
      {
        "idpGroup": "compliance-team",
        "role": "compliance_officer"
      },
      {
        "idpGroup": "operations",
        "role": "operator"
      }
    ],
    "defaultRole": "viewer"
  }'
```

### Available Roles

| Role | Permissions |
|------|-------------|
| `admin` | Full system access |
| `compliance_officer` | Compliance, KYC, AML management |
| `operator` | Transaction monitoring, support |
| `finance` | Financial reports, reconciliation |
| `viewer` | Read-only access |

---

## Multi-Factor Authentication

### Enforce MFA

RampOS can enforce MFA by checking IdP claims:

```bash
# Require MFA for all users
SSO_REQUIRE_MFA=true
SSO_MFA_CLAIM=amr
SSO_MFA_VALUES=mfa,otp,sms

# Or require MFA for specific roles
SSO_MFA_REQUIRED_ROLES=admin,compliance_officer
```

### MFA with Okta

1. Configure MFA in Okta
2. Create a sign-on policy requiring MFA
3. Okta includes `amr` claim with authentication methods

### MFA with Azure AD

1. Enable Azure MFA
2. Create Conditional Access policy
3. Configure RampOS:
```bash
SSO_MFA_CLAIM=amr
SSO_MFA_VALUES=mfa
```

---

## Session Management

### Session Configuration

```bash
# Session settings
SSO_SESSION_LIFETIME=8h
SSO_SESSION_REFRESH_ENABLED=true
SSO_SESSION_REFRESH_BEFORE=30m
SSO_SESSION_ABSOLUTE_TIMEOUT=24h

# Idle timeout
SSO_SESSION_IDLE_TIMEOUT=1h
SSO_SESSION_IDLE_ENABLED=true
```

### Force Re-authentication

```bash
# Require fresh auth for sensitive operations
SSO_STEP_UP_ENABLED=true
SSO_STEP_UP_MAX_AGE=5m
SSO_STEP_UP_OPERATIONS=kyc_approval,large_transaction,config_change
```

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Redirect loop | Incorrect callback URL | Verify `OIDC_REDIRECT_URI` matches IdP config |
| Invalid signature | Clock skew | Sync server time with NTP |
| Missing claims | Scope not requested | Add required scopes to `OIDC_SCOPES` |
| User not authorized | Group mapping failed | Check `SSO_ADMIN_GROUPS` configuration |

### Debug Logging

Enable SSO debug logging:

```bash
RUST_LOG=info,rampos_sso=debug
```

### Test SSO Configuration

```bash
# Validate OIDC configuration
curl https://api.your-domain.com/auth/oidc/.well-known/openid-configuration

# Validate SAML metadata
curl https://api.your-domain.com/auth/saml/metadata

# Test authentication flow
# Browser: https://api.your-domain.com/auth/login
```

### Common Error Messages

| Error | Meaning | Solution |
|-------|---------|----------|
| `invalid_client` | Client ID/secret mismatch | Verify credentials in IdP |
| `invalid_redirect_uri` | Callback URL not registered | Add URL to IdP allowed callbacks |
| `invalid_signature` | SAML signature verification failed | Check IdP certificate |
| `user_not_found` | User doesn't exist in RampOS | Enable auto-provisioning |

---

## Security Best Practices

1. **Use HTTPS everywhere** - Never use HTTP for SSO endpoints
2. **Rotate secrets regularly** - Update client secrets every 90 days
3. **Limit scopes** - Only request necessary scopes
4. **Enable MFA** - Require MFA for all users
5. **Monitor auth logs** - Alert on failed authentication attempts
6. **Use short session lifetimes** - 8 hours max for active sessions
7. **Implement SLO** - Ensure proper logout from both RampOS and IdP

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
