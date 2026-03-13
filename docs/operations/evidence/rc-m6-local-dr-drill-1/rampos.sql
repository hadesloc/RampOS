--
-- PostgreSQL database dump
--

\restrict AbNcwVUvibGCquh2BjBwf6v6t1zdOaRhhpddb3QjxcNh05SRx1mesGFfR7h5vEi

-- Dumped from database version 16.11
-- Dumped by pg_dump version 16.11

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: uuid-ossp; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;


--
-- Name: EXTENSION "uuid-ossp"; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION "uuid-ossp" IS 'generate universally unique identifiers (UUIDs)';


--
-- Name: aggregation_type; Type: TYPE; Schema: public; Owner: rampos
--

CREATE TYPE public.aggregation_type AS ENUM (
    'sum',
    'max',
    'unique_count'
);


ALTER TYPE public.aggregation_type OWNER TO rampos;

--
-- Name: billing_period; Type: TYPE; Schema: public; Owner: rampos
--

CREATE TYPE public.billing_period AS ENUM (
    'monthly',
    'yearly'
);


ALTER TYPE public.billing_period OWNER TO rampos;

--
-- Name: compliance_event_type; Type: TYPE; Schema: public; Owner: rampos
--

CREATE TYPE public.compliance_event_type AS ENUM (
    'compliance_decision',
    'document_submitted',
    'rule_changed',
    'user_action',
    'kyc_tier_change',
    'transaction_approval',
    'transaction_rejection',
    'aml_rule_modification',
    'sar_submission',
    'ctr_submission',
    'license_status_change',
    'sanctions_check',
    'pep_check',
    'travel_rule_policy_evaluated',
    'travel_rule_disclosure_updated',
    'travel_rule_exception_queued',
    'rescreening_run_completed',
    'rescreening_alert_queued',
    'rescreening_restriction_applied'
);


ALTER TYPE public.compliance_event_type OWNER TO rampos;

--
-- Name: domain_status; Type: TYPE; Schema: public; Owner: rampos
--

CREATE TYPE public.domain_status AS ENUM (
    'pending_dns_verification',
    'pending_ssl',
    'provisioning_ssl',
    'active',
    'expiring_soon',
    'expired',
    'dns_verification_failed',
    'ssl_provisioning_failed',
    'disabled'
);


ALTER TYPE public.domain_status OWNER TO rampos;

--
-- Name: invoice_status; Type: TYPE; Schema: public; Owner: rampos
--

CREATE TYPE public.invoice_status AS ENUM (
    'draft',
    'open',
    'paid',
    'void',
    'uncollectible'
);


ALTER TYPE public.invoice_status OWNER TO rampos;

--
-- Name: meter_type; Type: TYPE; Schema: public; Owner: rampos
--

CREATE TYPE public.meter_type AS ENUM (
    'api_calls',
    'transaction_volume',
    'active_users',
    'storage_gb'
);


ALTER TYPE public.meter_type OWNER TO rampos;

--
-- Name: sso_protocol; Type: TYPE; Schema: public; Owner: rampos
--

CREATE TYPE public.sso_protocol AS ENUM (
    'oidc',
    'saml2'
);


ALTER TYPE public.sso_protocol OWNER TO rampos;

--
-- Name: sso_provider_type; Type: TYPE; Schema: public; Owner: rampos
--

CREATE TYPE public.sso_provider_type AS ENUM (
    'okta',
    'azure_ad',
    'google',
    'auth0',
    'onelogin',
    'custom'
);


ALTER TYPE public.sso_provider_type OWNER TO rampos;

--
-- Name: append_state_history(); Type: FUNCTION; Schema: public; Owner: rampos
--

CREATE FUNCTION public.append_state_history() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF OLD.state IS DISTINCT FROM NEW.state THEN
        NEW.state_history = OLD.state_history || jsonb_build_object(
            'from', OLD.state,
            'to', NEW.state,
            'at', NOW()
        );
    END IF;
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.append_state_history() OWNER TO rampos;

--
-- Name: check_vnd_transaction_limit(character varying, character varying, numeric); Type: FUNCTION; Schema: public; Owner: rampos
--

CREATE FUNCTION public.check_vnd_transaction_limit(p_tenant_id character varying, p_user_id character varying, p_amount_vnd numeric) RETURNS jsonb
    LANGUAGE plpgsql STABLE
    AS $$
DECLARE
    v_user RECORD;
    v_limits RECORD;
    v_config RECORD;
    v_daily_used DECIMAL(20, 2);
    v_monthly_used DECIMAL(20, 2);
    v_single_limit DECIMAL(20, 2);
    v_daily_limit DECIMAL(20, 2);
    v_monthly_limit DECIMAL(20, 2);
    v_result JSONB;
BEGIN
    -- Get user info
    SELECT kyc_tier INTO v_user
    FROM users
    WHERE tenant_id = p_tenant_id AND id = p_user_id;

    IF NOT FOUND THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'USER_NOT_FOUND',
            'message', 'User not found'
        );
    END IF;

    -- Tier 0 cannot transact
    IF v_user.kyc_tier = 0 THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'TIER_NOT_ALLOWED',
            'message', 'Tier 0 users are not allowed to perform transactions'
        );
    END IF;

    -- Get custom limits or use defaults
    SELECT
        COALESCE(custom_single_limit_vnd,
            CASE v_user.kyc_tier
                WHEN 1 THEN 50000000  -- 50M
                WHEN 2 THEN 200000000 -- 200M
                WHEN 3 THEN 1000000000 -- 1B
                ELSE 0
            END),
        COALESCE(custom_daily_limit_vnd,
            CASE v_user.kyc_tier
                WHEN 1 THEN 100000000   -- 100M
                WHEN 2 THEN 500000000   -- 500M
                WHEN 3 THEN 9999999999999 -- Unlimited
                ELSE 0
            END),
        COALESCE(custom_monthly_limit_vnd,
            CASE v_user.kyc_tier
                WHEN 1 THEN 1000000000    -- 1B
                WHEN 2 THEN 5000000000    -- 5B
                WHEN 3 THEN 9999999999999 -- Unlimited
                ELSE 0
            END)
    INTO v_single_limit, v_daily_limit, v_monthly_limit
    FROM user_transaction_limits
    WHERE tenant_id = p_tenant_id AND user_id = p_user_id;

    -- Use defaults if no custom limits
    IF NOT FOUND THEN
        v_single_limit := CASE v_user.kyc_tier
            WHEN 1 THEN 50000000
            WHEN 2 THEN 200000000
            WHEN 3 THEN 1000000000
            ELSE 0
        END;
        v_daily_limit := CASE v_user.kyc_tier
            WHEN 1 THEN 100000000
            WHEN 2 THEN 500000000
            WHEN 3 THEN 9999999999999
            ELSE 0
        END;
        v_monthly_limit := CASE v_user.kyc_tier
            WHEN 1 THEN 1000000000
            WHEN 2 THEN 5000000000
            WHEN 3 THEN 9999999999999
            ELSE 0
        END;
    END IF;

    -- Check single transaction limit
    IF p_amount_vnd > v_single_limit THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'SINGLE_LIMIT_EXCEEDED',
            'message', format('Amount %s VND exceeds single transaction limit of %s VND', p_amount_vnd, v_single_limit),
            'limit', v_single_limit,
            'requested', p_amount_vnd
        );
    END IF;

    -- Get current usage
    v_daily_used := get_user_daily_used_vnd(p_tenant_id, p_user_id);
    v_monthly_used := get_user_monthly_used_vnd(p_tenant_id, p_user_id);

    -- Check daily limit
    IF (v_daily_used + p_amount_vnd) > v_daily_limit THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'DAILY_LIMIT_EXCEEDED',
            'message', format('Daily limit exceeded. Used: %s VND, Requested: %s VND, Limit: %s VND',
                v_daily_used, p_amount_vnd, v_daily_limit),
            'used', v_daily_used,
            'limit', v_daily_limit,
            'requested', p_amount_vnd
        );
    END IF;

    -- Check monthly limit
    IF (v_monthly_used + p_amount_vnd) > v_monthly_limit THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'MONTHLY_LIMIT_EXCEEDED',
            'message', format('Monthly limit exceeded. Used: %s VND, Requested: %s VND, Limit: %s VND',
                v_monthly_used, p_amount_vnd, v_monthly_limit),
            'used', v_monthly_used,
            'limit', v_monthly_limit,
            'requested', p_amount_vnd
        );
    END IF;

    -- All checks passed
    RETURN jsonb_build_object(
        'approved', true,
        'daily_remaining', v_daily_limit - v_daily_used - p_amount_vnd,
        'monthly_remaining', v_monthly_limit - v_monthly_used - p_amount_vnd,
        'daily_used', v_daily_used,
        'monthly_used', v_monthly_used,
        'daily_limit', v_daily_limit,
        'monthly_limit', v_monthly_limit
    );
END;
$$;


ALTER FUNCTION public.check_vnd_transaction_limit(p_tenant_id character varying, p_user_id character varying, p_amount_vnd numeric) OWNER TO rampos;

--
-- Name: FUNCTION check_vnd_transaction_limit(p_tenant_id character varying, p_user_id character varying, p_amount_vnd numeric); Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON FUNCTION public.check_vnd_transaction_limit(p_tenant_id character varying, p_user_id character varying, p_amount_vnd numeric) IS 'Check if a transaction is within VND limits for a user';


--
-- Name: get_user_daily_used_vnd(character varying, character varying, date); Type: FUNCTION; Schema: public; Owner: rampos
--

CREATE FUNCTION public.get_user_daily_used_vnd(p_tenant_id character varying, p_user_id character varying, p_date date DEFAULT NULL::date) RETURNS numeric
    LANGUAGE plpgsql STABLE
    AS $$
DECLARE
    v_total DECIMAL(20, 2);
    v_date DATE;
BEGIN
    -- Use provided date or current Vietnam date
    v_date := COALESCE(p_date, (NOW() AT TIME ZONE 'Asia/Ho_Chi_Minh')::DATE);

    SELECT COALESCE(SUM(amount_vnd), 0)
    INTO v_total
    FROM transaction_limit_history
    WHERE tenant_id = p_tenant_id
      AND user_id = p_user_id
      AND vietnam_date = v_date;

    RETURN v_total;
END;
$$;


ALTER FUNCTION public.get_user_daily_used_vnd(p_tenant_id character varying, p_user_id character varying, p_date date) OWNER TO rampos;

--
-- Name: FUNCTION get_user_daily_used_vnd(p_tenant_id character varying, p_user_id character varying, p_date date); Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON FUNCTION public.get_user_daily_used_vnd(p_tenant_id character varying, p_user_id character varying, p_date date) IS 'Calculate total VND amount used by a user on a specific date';


--
-- Name: get_user_monthly_used_vnd(character varying, character varying, character varying); Type: FUNCTION; Schema: public; Owner: rampos
--

CREATE FUNCTION public.get_user_monthly_used_vnd(p_tenant_id character varying, p_user_id character varying, p_month character varying DEFAULT NULL::character varying) RETURNS numeric
    LANGUAGE plpgsql STABLE
    AS $$
DECLARE
    v_total DECIMAL(20, 2);
    v_month VARCHAR(7);
BEGIN
    -- Use provided month or current Vietnam month
    v_month := COALESCE(p_month, TO_CHAR(NOW() AT TIME ZONE 'Asia/Ho_Chi_Minh', 'YYYY-MM'));

    SELECT COALESCE(SUM(amount_vnd), 0)
    INTO v_total
    FROM transaction_limit_history
    WHERE tenant_id = p_tenant_id
      AND user_id = p_user_id
      AND vietnam_month = v_month;

    RETURN v_total;
END;
$$;


ALTER FUNCTION public.get_user_monthly_used_vnd(p_tenant_id character varying, p_user_id character varying, p_month character varying) OWNER TO rampos;

--
-- Name: FUNCTION get_user_monthly_used_vnd(p_tenant_id character varying, p_user_id character varying, p_month character varying); Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON FUNCTION public.get_user_monthly_used_vnd(p_tenant_id character varying, p_user_id character varying, p_month character varying) IS 'Calculate total VND amount used by a user in a specific month';


--
-- Name: prevent_audit_modification(); Type: FUNCTION; Schema: public; Owner: rampos
--

CREATE FUNCTION public.prevent_audit_modification() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    RAISE EXCEPTION 'Compliance audit log is immutable - updates and deletes are not allowed';
END;
$$;


ALTER FUNCTION public.prevent_audit_modification() OWNER TO rampos;

--
-- Name: record_transaction_for_limits(character varying, character varying, character varying, character varying, numeric); Type: FUNCTION; Schema: public; Owner: rampos
--

CREATE FUNCTION public.record_transaction_for_limits(p_tenant_id character varying, p_user_id character varying, p_intent_id character varying, p_transaction_type character varying, p_amount_vnd numeric) RETURNS bigint
    LANGUAGE plpgsql
    AS $$
DECLARE
    v_id BIGINT;
BEGIN
    INSERT INTO transaction_limit_history (
        tenant_id, user_id, intent_id, transaction_type, amount_vnd
    ) VALUES (
        p_tenant_id, p_user_id, p_intent_id, p_transaction_type, p_amount_vnd
    )
    RETURNING id INTO v_id;

    RETURN v_id;
END;
$$;


ALTER FUNCTION public.record_transaction_for_limits(p_tenant_id character varying, p_user_id character varying, p_intent_id character varying, p_transaction_type character varying, p_amount_vnd numeric) OWNER TO rampos;

--
-- Name: FUNCTION record_transaction_for_limits(p_tenant_id character varying, p_user_id character varying, p_intent_id character varying, p_transaction_type character varying, p_amount_vnd numeric); Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON FUNCTION public.record_transaction_for_limits(p_tenant_id character varying, p_user_id character varying, p_intent_id character varying, p_transaction_type character varying, p_amount_vnd numeric) IS 'Record a completed transaction for limit tracking';


--
-- Name: update_updated_at(); Type: FUNCTION; Schema: public; Owner: rampos
--

CREATE FUNCTION public.update_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.update_updated_at() OWNER TO rampos;

--
-- Name: update_updated_at_column(); Type: FUNCTION; Schema: public; Owner: rampos
--

CREATE FUNCTION public.update_updated_at_column() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.update_updated_at_column() OWNER TO rampos;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO rampos;

--
-- Name: account_balances; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.account_balances (
    id integer NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64),
    account_type character varying(64) NOT NULL,
    currency character varying(16) NOT NULL,
    balance numeric(30,8) DEFAULT 0 NOT NULL,
    last_entry_id character varying(64),
    last_sequence bigint,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY public.account_balances FORCE ROW LEVEL SECURITY;


ALTER TABLE public.account_balances OWNER TO rampos;

--
-- Name: account_balances_id_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.account_balances_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_balances_id_seq OWNER TO rampos;

--
-- Name: account_balances_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.account_balances_id_seq OWNED BY public.account_balances.id;


--
-- Name: aml_cases; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.aml_cases (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64),
    intent_id character varying(64),
    case_type character varying(64) NOT NULL,
    severity character varying(16) NOT NULL,
    status character varying(32) DEFAULT 'OPEN'::character varying NOT NULL,
    rule_id character varying(64),
    rule_name character varying(255),
    detection_data jsonb NOT NULL,
    assigned_to character varying(64),
    resolution text,
    resolved_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY public.aml_cases FORCE ROW LEVEL SECURITY;


ALTER TABLE public.aml_cases OWNER TO rampos;

--
-- Name: aml_rule_versions; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.aml_rule_versions (
    id uuid NOT NULL,
    tenant_id character varying(64) NOT NULL,
    version_number integer NOT NULL,
    rules_json jsonb NOT NULL,
    is_active boolean DEFAULT false,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by character varying(255),
    activated_at timestamp with time zone,
    version_state text DEFAULT 'DRAFT'::text NOT NULL,
    version_label text,
    parent_version_id uuid,
    version_metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    scorer_config jsonb,
    decision_thresholds jsonb,
    CONSTRAINT aml_rule_versions_decision_thresholds_object CHECK (((decision_thresholds IS NULL) OR (jsonb_typeof(decision_thresholds) = 'object'::text))),
    CONSTRAINT aml_rule_versions_scorer_config_object CHECK (((scorer_config IS NULL) OR (jsonb_typeof(scorer_config) = 'object'::text))),
    CONSTRAINT aml_rule_versions_version_metadata_object CHECK ((jsonb_typeof(version_metadata) = 'object'::text)),
    CONSTRAINT aml_rule_versions_version_state_check CHECK ((version_state = ANY (ARRAY['DRAFT'::text, 'ACTIVE'::text, 'SHADOW'::text, 'ARCHIVED'::text])))
);

ALTER TABLE ONLY public.aml_rule_versions FORCE ROW LEVEL SECURITY;


ALTER TABLE public.aml_rule_versions OWNER TO rampos;

--
-- Name: audit_log; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.audit_log (
    id bigint NOT NULL,
    tenant_id character varying(64) NOT NULL,
    actor_type character varying(32) NOT NULL,
    actor_id character varying(64),
    action character varying(64) NOT NULL,
    resource_type character varying(64) NOT NULL,
    resource_id character varying(64),
    details jsonb DEFAULT '{}'::jsonb NOT NULL,
    ip_address inet,
    user_agent text,
    request_id character varying(64),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    prev_hash character varying(64),
    entry_hash character varying(64) NOT NULL
);

ALTER TABLE ONLY public.audit_log FORCE ROW LEVEL SECURITY;


ALTER TABLE public.audit_log OWNER TO rampos;

--
-- Name: audit_log_id_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.audit_log_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_log_id_seq OWNER TO rampos;

--
-- Name: audit_log_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.audit_log_id_seq OWNED BY public.audit_log.id;


--
-- Name: bank_confirmations; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.bank_confirmations (
    id character varying(64) DEFAULT ('BC_'::text || (gen_random_uuid())::text) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    provider character varying(64) NOT NULL,
    reference_code character varying(128) NOT NULL,
    bank_reference character varying(128),
    bank_tx_id character varying(128),
    amount numeric(30,8) NOT NULL,
    currency character varying(16) DEFAULT 'VND'::character varying NOT NULL,
    sender_account character varying(64),
    sender_name character varying(255),
    receiver_account character varying(64),
    receiver_name character varying(255),
    status character varying(32) DEFAULT 'PENDING'::character varying NOT NULL,
    matched_intent_id character varying(64),
    matched_at timestamp with time zone,
    webhook_received_at timestamp with time zone DEFAULT now() NOT NULL,
    webhook_signature character varying(512),
    webhook_signature_verified boolean DEFAULT false NOT NULL,
    raw_payload jsonb NOT NULL,
    processing_notes text,
    transaction_time timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT bank_confirmations_status_check CHECK (((status)::text = ANY ((ARRAY['PENDING'::character varying, 'MATCHED'::character varying, 'UNMATCHED'::character varying, 'DUPLICATE'::character varying, 'REJECTED'::character varying])::text[])))
);

ALTER TABLE ONLY public.bank_confirmations FORCE ROW LEVEL SECURITY;


ALTER TABLE public.bank_confirmations OWNER TO rampos;

--
-- Name: TABLE bank_confirmations; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.bank_confirmations IS 'Stores incoming bank webhook confirmations for pay-in matching';


--
-- Name: COLUMN bank_confirmations.reference_code; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.bank_confirmations.reference_code IS 'Our reference code used to match with pending intents';


--
-- Name: COLUMN bank_confirmations.bank_tx_id; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.bank_confirmations.bank_tx_id IS 'Bank internal transaction ID for duplicate detection';


--
-- Name: COLUMN bank_confirmations.raw_payload; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.bank_confirmations.raw_payload IS 'Original webhook payload for audit trail';


--
-- Name: bank_webhook_secrets; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.bank_webhook_secrets (
    id integer NOT NULL,
    tenant_id character varying(64) NOT NULL,
    provider character varying(64) NOT NULL,
    secret_encrypted bytea NOT NULL,
    algorithm character varying(32) DEFAULT 'HMAC-SHA256'::character varying NOT NULL,
    header_name character varying(64) DEFAULT 'X-Signature'::character varying NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY public.bank_webhook_secrets FORCE ROW LEVEL SECURITY;


ALTER TABLE public.bank_webhook_secrets OWNER TO rampos;

--
-- Name: TABLE bank_webhook_secrets; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.bank_webhook_secrets IS 'Stores webhook secrets for signature verification per provider';


--
-- Name: bank_webhook_secrets_id_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.bank_webhook_secrets_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.bank_webhook_secrets_id_seq OWNER TO rampos;

--
-- Name: bank_webhook_secrets_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.bank_webhook_secrets_id_seq OWNED BY public.bank_webhook_secrets.id;


--
-- Name: billing_meters; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.billing_meters (
    id character varying(64) NOT NULL,
    slug character varying(64) NOT NULL,
    name character varying(255) NOT NULL,
    type public.meter_type NOT NULL,
    aggregation public.aggregation_type NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.billing_meters OWNER TO rampos;

--
-- Name: case_notes; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.case_notes (
    id uuid NOT NULL,
    case_id character varying(255) NOT NULL,
    author_id character varying(255),
    content text NOT NULL,
    note_type character varying(50) NOT NULL,
    is_internal boolean DEFAULT true,
    created_at timestamp with time zone DEFAULT now(),
    tenant_id character varying(64) NOT NULL
);

ALTER TABLE ONLY public.case_notes FORCE ROW LEVEL SECURITY;


ALTER TABLE public.case_notes OWNER TO rampos;

--
-- Name: compliance_audit_log; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.compliance_audit_log (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id character varying(64) NOT NULL,
    event_type public.compliance_event_type NOT NULL,
    actor_id character varying(64),
    actor_type character varying(32) DEFAULT 'SYSTEM'::character varying NOT NULL,
    action_details jsonb DEFAULT '{}'::jsonb NOT NULL,
    resource_type character varying(64),
    resource_id character varying(64),
    sequence_number bigint NOT NULL,
    previous_hash character varying(64),
    current_hash character varying(64) NOT NULL,
    ip_address inet,
    user_agent text,
    request_id character varying(64),
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.compliance_audit_log OWNER TO rampos;

--
-- Name: TABLE compliance_audit_log; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.compliance_audit_log IS 'Immutable compliance audit trail with hash chain for regulatory inspections.
     This table is append-only - updates and deletes are prevented by triggers and RLS.';


--
-- Name: COLUMN compliance_audit_log.previous_hash; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.compliance_audit_log.previous_hash IS 'Hash of the previous record in the chain for this tenant. NULL for the first record.';


--
-- Name: COLUMN compliance_audit_log.current_hash; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.compliance_audit_log.current_hash IS 'SHA256 hash of (event_type + actor_id + action_details + resource_id + created_at + previous_hash)';


--
-- Name: compliance_audit_log_sequence_number_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.compliance_audit_log_sequence_number_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.compliance_audit_log_sequence_number_seq OWNER TO rampos;

--
-- Name: compliance_audit_log_sequence_number_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.compliance_audit_log_sequence_number_seq OWNED BY public.compliance_audit_log.sequence_number;


--
-- Name: compliance_rescreening_runs; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.compliance_rescreening_runs (
    id text NOT NULL,
    tenant_id text NOT NULL,
    user_id text NOT NULL,
    trigger_kind text DEFAULT 'SCHEDULED'::text NOT NULL,
    status text DEFAULT 'PENDING'::text NOT NULL,
    priority text DEFAULT 'MEDIUM'::text NOT NULL,
    restriction_status text DEFAULT 'NONE'::text NOT NULL,
    alert_codes jsonb DEFAULT '[]'::jsonb NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL,
    scheduled_for timestamp with time zone NOT NULL,
    executed_at timestamp with time zone,
    next_run_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT compliance_rescreening_runs_alert_codes_array CHECK ((jsonb_typeof(alert_codes) = 'array'::text)),
    CONSTRAINT compliance_rescreening_runs_details_object CHECK ((jsonb_typeof(details) = 'object'::text)),
    CONSTRAINT compliance_rescreening_runs_priority_check CHECK ((priority = ANY (ARRAY['LOW'::text, 'MEDIUM'::text, 'HIGH'::text, 'CRITICAL'::text]))),
    CONSTRAINT compliance_rescreening_runs_restriction_check CHECK ((restriction_status = ANY (ARRAY['NONE'::text, 'REVIEW_REQUIRED'::text, 'RESTRICTED'::text]))),
    CONSTRAINT compliance_rescreening_runs_status_check CHECK ((status = ANY (ARRAY['PENDING'::text, 'ALERTED'::text, 'RESTRICTED'::text, 'CLEARED'::text]))),
    CONSTRAINT compliance_rescreening_runs_trigger_kind_check CHECK ((trigger_kind = ANY (ARRAY['SCHEDULED'::text, 'WATCHLIST_DELTA'::text, 'DOCUMENT_EXPIRY'::text])))
);


ALTER TABLE public.compliance_rescreening_runs OWNER TO rampos;

--
-- Name: compliance_transactions; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.compliance_transactions (
    id uuid NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    intent_id character varying(64) NOT NULL,
    transaction_type character varying(32) NOT NULL,
    amount_vnd numeric(30,8) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY public.compliance_transactions FORCE ROW LEVEL SECURITY;


ALTER TABLE public.compliance_transactions OWNER TO rampos;

--
-- Name: config_bundle_exports; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.config_bundle_exports (
    id text NOT NULL,
    tenant_id text,
    tenant_name text NOT NULL,
    action_mode text DEFAULT 'whitelisted_only'::text NOT NULL,
    sections jsonb DEFAULT '[]'::jsonb NOT NULL,
    payload jsonb DEFAULT '{}'::jsonb NOT NULL,
    approval_status text DEFAULT 'approved'::text NOT NULL,
    rollout_scope jsonb DEFAULT '{}'::jsonb NOT NULL,
    provenance jsonb DEFAULT '{}'::jsonb NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.config_bundle_exports OWNER TO rampos;

--
-- Name: corridor_compliance_hooks; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.corridor_compliance_hooks (
    id text NOT NULL,
    corridor_pack_id text NOT NULL,
    hook_kind text NOT NULL,
    provider_key text,
    required boolean DEFAULT false NOT NULL,
    config jsonb DEFAULT '{}'::jsonb NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT corridor_compliance_hooks_config_object CHECK ((jsonb_typeof(config) = 'object'::text)),
    CONSTRAINT corridor_compliance_hooks_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text))
);


ALTER TABLE public.corridor_compliance_hooks OWNER TO rampos;

--
-- Name: corridor_cutoff_policies; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.corridor_cutoff_policies (
    id text NOT NULL,
    corridor_pack_id text NOT NULL,
    timezone text NOT NULL,
    cutoff_windows jsonb DEFAULT '[]'::jsonb NOT NULL,
    holiday_calendar text,
    retry_rule text,
    exception_policy text,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT corridor_cutoff_policies_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT corridor_cutoff_policies_windows_array CHECK ((jsonb_typeof(cutoff_windows) = 'array'::text))
);


ALTER TABLE public.corridor_cutoff_policies OWNER TO rampos;

--
-- Name: corridor_eligibility_rules; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.corridor_eligibility_rules (
    id text NOT NULL,
    corridor_pack_id text NOT NULL,
    partner_id text,
    entity_type text,
    method_family text,
    amount_bounds jsonb DEFAULT '{}'::jsonb NOT NULL,
    compliance_requirements jsonb DEFAULT '[]'::jsonb NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT corridor_eligibility_rules_amount_bounds_object CHECK ((jsonb_typeof(amount_bounds) = 'object'::text)),
    CONSTRAINT corridor_eligibility_rules_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT corridor_eligibility_rules_requirements_array CHECK ((jsonb_typeof(compliance_requirements) = 'array'::text))
);


ALTER TABLE public.corridor_eligibility_rules OWNER TO rampos;

--
-- Name: corridor_fee_profiles; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.corridor_fee_profiles (
    id text NOT NULL,
    corridor_pack_id text NOT NULL,
    fee_currency text NOT NULL,
    base_fee numeric(20,8),
    fx_spread_bps integer,
    liquidity_cost_bps integer,
    surcharge_bps integer,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT corridor_fee_profiles_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text))
);


ALTER TABLE public.corridor_fee_profiles OWNER TO rampos;

--
-- Name: corridor_pack_endpoints; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.corridor_pack_endpoints (
    id text NOT NULL,
    corridor_pack_id text NOT NULL,
    endpoint_role text NOT NULL,
    partner_id text,
    provider_key text,
    adapter_key text,
    entity_type text NOT NULL,
    rail text NOT NULL,
    method_family text,
    settlement_mode text,
    instrument_family text,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT corridor_pack_endpoints_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT corridor_pack_endpoints_role_check CHECK ((endpoint_role = ANY (ARRAY['source'::text, 'destination'::text])))
);


ALTER TABLE public.corridor_pack_endpoints OWNER TO rampos;

--
-- Name: corridor_packs; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.corridor_packs (
    id text NOT NULL,
    tenant_id text,
    corridor_code text NOT NULL,
    source_market text NOT NULL,
    destination_market text NOT NULL,
    source_currency text NOT NULL,
    destination_currency text NOT NULL,
    settlement_direction text NOT NULL,
    fee_model text DEFAULT 'shared'::text NOT NULL,
    lifecycle_state text DEFAULT 'draft'::text NOT NULL,
    rollout_state text DEFAULT 'planned'::text NOT NULL,
    eligibility_state text DEFAULT 'restricted'::text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT corridor_packs_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text))
);


ALTER TABLE public.corridor_packs OWNER TO rampos;

--
-- Name: corridor_rollout_scopes; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.corridor_rollout_scopes (
    id text NOT NULL,
    corridor_pack_id text NOT NULL,
    tenant_id text,
    environment text DEFAULT 'sandbox'::text NOT NULL,
    geography text,
    method_family text,
    rollout_state text DEFAULT 'planned'::text NOT NULL,
    approval_reference text,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT corridor_rollout_scopes_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text))
);


ALTER TABLE public.corridor_rollout_scopes OWNER TO rampos;

--
-- Name: credential_references; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.credential_references (
    id text NOT NULL,
    partner_id text NOT NULL,
    credential_kind text NOT NULL,
    locator text NOT NULL,
    environment text DEFAULT 'sandbox'::text NOT NULL,
    approval_reference text,
    rotation_metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.credential_references OWNER TO rampos;

--
-- Name: custom_domains; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.custom_domains (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    domain character varying(255) NOT NULL,
    status public.domain_status DEFAULT 'pending_dns_verification'::public.domain_status NOT NULL,
    dns_verification_token character varying(255),
    dns_verification_record character varying(255),
    ssl_certificate jsonb,
    health_check_path character varying(255) DEFAULT '/health'::character varying NOT NULL,
    last_health_check jsonb,
    is_primary boolean DEFAULT false NOT NULL,
    custom_headers jsonb DEFAULT '{}'::jsonb,
    redirects jsonb DEFAULT '[]'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.custom_domains OWNER TO rampos;

--
-- Name: daily_usage; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.daily_usage (
    tenant_id character varying(64) NOT NULL,
    meter_slug character varying(64) NOT NULL,
    date date NOT NULL,
    total_amount numeric(36,18) DEFAULT 0 NOT NULL
);


ALTER TABLE public.daily_usage OWNER TO rampos;

--
-- Name: identity_providers; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.identity_providers (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    name character varying(255) NOT NULL,
    slug character varying(255) NOT NULL,
    type public.sso_provider_type NOT NULL,
    protocol public.sso_protocol NOT NULL,
    is_enabled boolean DEFAULT true NOT NULL,
    config jsonb NOT NULL,
    role_mappings jsonb DEFAULT '[]'::jsonb NOT NULL,
    default_role character varying(64) DEFAULT 'viewer'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.identity_providers OWNER TO rampos;

--
-- Name: intents; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.intents (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    intent_type character varying(32) NOT NULL,
    state character varying(64) NOT NULL,
    state_history jsonb DEFAULT '[]'::jsonb NOT NULL,
    amount numeric(30,8) NOT NULL,
    currency character varying(16) NOT NULL,
    actual_amount numeric(30,8),
    rails_provider character varying(64),
    reference_code character varying(64),
    bank_tx_id character varying(128),
    chain_id character varying(32),
    tx_hash character varying(128),
    from_address character varying(128),
    to_address character varying(128),
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    idempotency_key character varying(128),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone,
    completed_at timestamp with time zone,
    CONSTRAINT intents_type_check CHECK (((intent_type)::text = ANY ((ARRAY['PAYIN_VND'::character varying, 'PAYOUT_VND'::character varying, 'TRADE_EXECUTED'::character varying, 'DEPOSIT_ONCHAIN'::character varying, 'WITHDRAW_ONCHAIN'::character varying])::text[])))
);

ALTER TABLE ONLY public.intents FORCE ROW LEVEL SECURITY;


ALTER TABLE public.intents OWNER TO rampos;

--
-- Name: invoices; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.invoices (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    period_start timestamp with time zone NOT NULL,
    period_end timestamp with time zone NOT NULL,
    status public.invoice_status DEFAULT 'draft'::public.invoice_status NOT NULL,
    currency character varying(3) NOT NULL,
    subtotal numeric(18,2) NOT NULL,
    tax numeric(18,2) DEFAULT 0 NOT NULL,
    total numeric(18,2) NOT NULL,
    line_items jsonb NOT NULL,
    due_date date,
    paid_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.invoices OWNER TO rampos;

--
-- Name: kyb_entities; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyb_entities (
    id text NOT NULL,
    tenant_id text NOT NULL,
    entity_type text NOT NULL,
    display_name text NOT NULL,
    jurisdiction text,
    status text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.kyb_entities OWNER TO rampos;

--
-- Name: kyb_evidence_packages; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyb_evidence_packages (
    id text NOT NULL,
    tenant_id text NOT NULL,
    institution_entity_id text NOT NULL,
    institution_legal_name text NOT NULL,
    provider_family text NOT NULL,
    provider_policy_id text,
    corridor_code text,
    review_status text DEFAULT 'pending'::text NOT NULL,
    review_notes text,
    export_status text DEFAULT 'not_exported'::text NOT NULL,
    export_artifact_uri text,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    exported_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.kyb_evidence_packages OWNER TO rampos;

--
-- Name: kyb_evidence_sources; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyb_evidence_sources (
    id text NOT NULL,
    package_id text NOT NULL,
    source_kind text NOT NULL,
    source_ref text NOT NULL,
    document_id text,
    collected_at timestamp with time zone DEFAULT now() NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.kyb_evidence_sources OWNER TO rampos;

--
-- Name: kyb_ownership_edges; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyb_ownership_edges (
    id text NOT NULL,
    tenant_id text NOT NULL,
    source_id text NOT NULL,
    target_id text NOT NULL,
    edge_type text NOT NULL,
    ownership_pct numeric(5,2),
    effective_from timestamp with time zone,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.kyb_ownership_edges OWNER TO rampos;

--
-- Name: kyb_ubo_evidence_links; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyb_ubo_evidence_links (
    id text NOT NULL,
    package_id text NOT NULL,
    owner_entity_id text NOT NULL,
    ownership_pct numeric(5,2),
    evidence_source_ref text,
    review_state text DEFAULT 'pending'::text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.kyb_ubo_evidence_links OWNER TO rampos;

--
-- Name: kyc_passport_acceptance_policies; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyc_passport_acceptance_policies (
    id text NOT NULL,
    tenant_id text NOT NULL,
    min_tier smallint DEFAULT 0 NOT NULL,
    max_age_days integer DEFAULT 30 NOT NULL,
    allowed_source_tenants jsonb DEFAULT '[]'::jsonb NOT NULL,
    requires_manual_review boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.kyc_passport_acceptance_policies OWNER TO rampos;

--
-- Name: kyc_passport_consent_grants; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyc_passport_consent_grants (
    id text NOT NULL,
    tenant_id text NOT NULL,
    passport_id text NOT NULL,
    target_tenant_id text NOT NULL,
    consent_status text NOT NULL,
    scope jsonb DEFAULT '{}'::jsonb NOT NULL,
    granted_at timestamp with time zone,
    revoked_at timestamp with time zone,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.kyc_passport_consent_grants OWNER TO rampos;

--
-- Name: kyc_passport_vault; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyc_passport_vault (
    id text NOT NULL,
    tenant_id text NOT NULL,
    user_id text NOT NULL,
    source_tenant_id text NOT NULL,
    status text NOT NULL,
    kyc_tier smallint DEFAULT 0 NOT NULL,
    fields_shared jsonb DEFAULT '[]'::jsonb NOT NULL,
    verified_at timestamp with time zone,
    expires_at timestamp with time zone,
    revoked_at timestamp with time zone,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.kyc_passport_vault OWNER TO rampos;

--
-- Name: kyc_records; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.kyc_records (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    tier smallint NOT NULL,
    provider character varying(64),
    provider_reference character varying(128),
    status character varying(32) NOT NULL,
    verification_data jsonb DEFAULT '{}'::jsonb NOT NULL,
    rejection_reason text,
    documents jsonb DEFAULT '[]'::jsonb,
    submitted_at timestamp with time zone DEFAULT now() NOT NULL,
    verified_at timestamp with time zone,
    expires_at timestamp with time zone
);

ALTER TABLE ONLY public.kyc_records FORCE ROW LEVEL SECURITY;


ALTER TABLE public.kyc_records OWNER TO rampos;

--
-- Name: ledger_entries; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.ledger_entries (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64),
    intent_id character varying(64) NOT NULL,
    transaction_id character varying(64) NOT NULL,
    account_type character varying(64) NOT NULL,
    direction character varying(8) NOT NULL,
    amount numeric(30,8) NOT NULL,
    currency character varying(16) NOT NULL,
    balance_after numeric(30,8) NOT NULL,
    sequence bigint NOT NULL,
    description text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT ledger_amount_positive CHECK ((amount >= (0)::numeric)),
    CONSTRAINT ledger_direction_check CHECK (((direction)::text = ANY ((ARRAY['DEBIT'::character varying, 'CREDIT'::character varying])::text[])))
);

ALTER TABLE ONLY public.ledger_entries FORCE ROW LEVEL SECURITY;


ALTER TABLE public.ledger_entries OWNER TO rampos;

--
-- Name: ledger_entries_sequence_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.ledger_entries_sequence_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.ledger_entries_sequence_seq OWNER TO rampos;

--
-- Name: ledger_entries_sequence_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.ledger_entries_sequence_seq OWNED BY public.ledger_entries.sequence;


--
-- Name: license_requirements; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.license_requirements (
    id character varying(64) NOT NULL,
    license_type_id character varying(64) NOT NULL,
    requirement_name character varying(255) NOT NULL,
    requirement_code character varying(64) NOT NULL,
    description text,
    is_mandatory boolean DEFAULT true NOT NULL,
    document_type character varying(64),
    validation_rules jsonb DEFAULT '{}'::jsonb,
    display_order integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.license_requirements OWNER TO rampos;

--
-- Name: license_submissions; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.license_submissions (
    id text NOT NULL,
    tenant_id text NOT NULL,
    requirement_id text NOT NULL,
    documents jsonb DEFAULT '[]'::jsonb NOT NULL,
    status text DEFAULT 'SUBMITTED'::text NOT NULL,
    submitted_by text NOT NULL,
    submitted_at timestamp with time zone DEFAULT now() NOT NULL,
    reviewed_at timestamp with time zone,
    reviewer_notes text
);


ALTER TABLE public.license_submissions OWNER TO rampos;

--
-- Name: TABLE license_submissions; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.license_submissions IS 'Document submissions from tenants for licensing applications';


--
-- Name: license_types; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.license_types (
    id character varying(64) NOT NULL,
    name character varying(255) NOT NULL,
    code character varying(32) NOT NULL,
    description text,
    jurisdiction character varying(64) DEFAULT 'VN'::character varying NOT NULL,
    regulatory_body character varying(255),
    is_active boolean DEFAULT true NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.license_types OWNER TO rampos;

--
-- Name: lp_reliability_snapshots; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.lp_reliability_snapshots (
    id text NOT NULL,
    tenant_id text NOT NULL,
    lp_id text NOT NULL,
    direction text DEFAULT 'OFFRAMP'::text NOT NULL,
    window_kind text DEFAULT 'ROLLING_30D'::text NOT NULL,
    window_started_at timestamp with time zone NOT NULL,
    window_ended_at timestamp with time zone NOT NULL,
    snapshot_version text DEFAULT 'v1'::text NOT NULL,
    quote_count integer DEFAULT 0 NOT NULL,
    fill_count integer DEFAULT 0 NOT NULL,
    reject_count integer DEFAULT 0 NOT NULL,
    settlement_count integer DEFAULT 0 NOT NULL,
    dispute_count integer DEFAULT 0 NOT NULL,
    fill_rate numeric(6,5) DEFAULT 0 NOT NULL,
    reject_rate numeric(6,5) DEFAULT 0 NOT NULL,
    dispute_rate numeric(6,5) DEFAULT 0 NOT NULL,
    avg_slippage_bps numeric(10,2) DEFAULT 0 NOT NULL,
    p95_settlement_latency_seconds integer DEFAULT 0 NOT NULL,
    reliability_score numeric(10,4),
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT lp_reliability_snapshots_direction_check CHECK ((direction = ANY (ARRAY['OFFRAMP'::text, 'ONRAMP'::text]))),
    CONSTRAINT lp_reliability_snapshots_dispute_count_check CHECK (((dispute_count >= 0) AND (dispute_count <= settlement_count))),
    CONSTRAINT lp_reliability_snapshots_dispute_rate_check CHECK (((dispute_rate >= (0)::numeric) AND (dispute_rate <= (1)::numeric))),
    CONSTRAINT lp_reliability_snapshots_fill_count_check CHECK (((fill_count >= 0) AND (fill_count <= quote_count))),
    CONSTRAINT lp_reliability_snapshots_fill_rate_check CHECK (((fill_rate >= (0)::numeric) AND (fill_rate <= (1)::numeric))),
    CONSTRAINT lp_reliability_snapshots_latency_p95_check CHECK ((p95_settlement_latency_seconds >= 0)),
    CONSTRAINT lp_reliability_snapshots_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT lp_reliability_snapshots_quote_count_check CHECK ((quote_count >= 0)),
    CONSTRAINT lp_reliability_snapshots_reject_count_check CHECK (((reject_count >= 0) AND (reject_count <= quote_count))),
    CONSTRAINT lp_reliability_snapshots_reject_rate_check CHECK (((reject_rate >= (0)::numeric) AND (reject_rate <= (1)::numeric))),
    CONSTRAINT lp_reliability_snapshots_settlement_count_check CHECK (((settlement_count >= 0) AND (settlement_count <= fill_count))),
    CONSTRAINT lp_reliability_snapshots_slippage_check CHECK ((avg_slippage_bps >= (0)::numeric)),
    CONSTRAINT lp_reliability_snapshots_window_kind_check CHECK ((window_kind = ANY (ARRAY['ROLLING_24H'::text, 'ROLLING_7D'::text, 'ROLLING_30D'::text, 'CALENDAR_DAY'::text]))),
    CONSTRAINT lp_reliability_snapshots_window_order CHECK ((window_started_at <= window_ended_at))
);


ALTER TABLE public.lp_reliability_snapshots OWNER TO rampos;

--
-- Name: magic_link_tokens; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.magic_link_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    token_hash bytea NOT NULL,
    email character varying(255) NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    used boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.magic_link_tokens OWNER TO rampos;

--
-- Name: offramp_intents; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.offramp_intents (
    id text NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id text NOT NULL,
    crypto_asset text NOT NULL,
    crypto_amount numeric NOT NULL,
    exchange_rate numeric NOT NULL,
    locked_rate_id text,
    fees jsonb DEFAULT '{}'::jsonb NOT NULL,
    net_vnd_amount numeric NOT NULL,
    gross_vnd_amount numeric NOT NULL,
    bank_account jsonb DEFAULT '{}'::jsonb NOT NULL,
    deposit_address text,
    tx_hash text,
    bank_reference text,
    state text DEFAULT 'QUOTE_CREATED'::text NOT NULL,
    state_history jsonb DEFAULT '[]'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    quote_expires_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.offramp_intents OWNER TO rampos;

--
-- Name: partner_approval_references; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.partner_approval_references (
    id text NOT NULL,
    tenant_id text,
    action_class text NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.partner_approval_references OWNER TO rampos;

--
-- Name: partner_capabilities; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.partner_capabilities (
    id text NOT NULL,
    partner_id text NOT NULL,
    capability_family text NOT NULL,
    environment text DEFAULT 'sandbox'::text NOT NULL,
    adapter_key text,
    provider_key text,
    supported_rails jsonb DEFAULT '[]'::jsonb NOT NULL,
    supported_methods jsonb DEFAULT '[]'::jsonb NOT NULL,
    approval_status text DEFAULT 'pending'::text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.partner_capabilities OWNER TO rampos;

--
-- Name: partner_health_signals; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.partner_health_signals (
    id text NOT NULL,
    partner_capability_id text NOT NULL,
    status text NOT NULL,
    source text NOT NULL,
    score integer,
    incident_summary text,
    evidence jsonb DEFAULT '{}'::jsonb NOT NULL,
    observed_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.partner_health_signals OWNER TO rampos;

--
-- Name: partner_rollout_scopes; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.partner_rollout_scopes (
    id text NOT NULL,
    partner_capability_id text NOT NULL,
    tenant_id text,
    environment text DEFAULT 'sandbox'::text NOT NULL,
    corridor_code text,
    geography text,
    method_family text,
    rollout_state text DEFAULT 'planned'::text NOT NULL,
    rollback_target text,
    approval_reference text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.partner_rollout_scopes OWNER TO rampos;

--
-- Name: partners; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.partners (
    id text NOT NULL,
    tenant_id text,
    partner_class text NOT NULL,
    code text NOT NULL,
    display_name text NOT NULL,
    legal_name text,
    market text,
    jurisdiction text,
    service_domain text NOT NULL,
    lifecycle_state text DEFAULT 'draft'::text NOT NULL,
    approval_status text DEFAULT 'pending'::text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.partners OWNER TO rampos;

--
-- Name: payment_method_capabilities; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.payment_method_capabilities (
    id text NOT NULL,
    corridor_pack_id text NOT NULL,
    partner_capability_id text,
    method_family text NOT NULL,
    funding_source text,
    settlement_direction text NOT NULL,
    presentment_model text,
    card_funding_enabled boolean DEFAULT false NOT NULL,
    policy_flags jsonb DEFAULT '{}'::jsonb NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.payment_method_capabilities OWNER TO rampos;

--
-- Name: portal_kyc_cases; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.portal_kyc_cases (
    id character varying(64) DEFAULT (gen_random_uuid())::text NOT NULL,
    user_id character varying(64) NOT NULL,
    tenant_id character varying(64) DEFAULT '00000000-0000-0000-0000-000000000001'::character varying NOT NULL,
    status character varying(32) DEFAULT 'PENDING'::character varying NOT NULL,
    tier smallint DEFAULT 1 NOT NULL,
    full_name character varying(200) NOT NULL,
    date_of_birth character varying(10) NOT NULL,
    document_type character varying(50) NOT NULL,
    document_number character varying(50),
    address text NOT NULL,
    reviewer_notes text,
    reviewed_at timestamp with time zone,
    submitted_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT portal_kyc_status_check CHECK (((status)::text = ANY ((ARRAY['PENDING'::character varying, 'APPROVED'::character varying, 'REJECTED'::character varying])::text[]))),
    CONSTRAINT portal_kyc_tier_check CHECK (((tier >= 0) AND (tier <= 3)))
);


ALTER TABLE public.portal_kyc_cases OWNER TO rampos;

--
-- Name: portal_kyc_documents; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.portal_kyc_documents (
    id character varying(64) DEFAULT (gen_random_uuid())::text NOT NULL,
    case_id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    document_type character varying(32) NOT NULL,
    filename character varying(255) NOT NULL,
    content_type character varying(100) NOT NULL,
    file_size bigint NOT NULL,
    file_path text NOT NULL,
    uploaded_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT portal_kyc_doc_type_check CHECK (((document_type)::text = ANY ((ARRAY['front'::character varying, 'back'::character varying, 'selfie'::character varying])::text[])))
);


ALTER TABLE public.portal_kyc_documents OWNER TO rampos;

--
-- Name: portal_users; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.portal_users (
    id character varying(64) NOT NULL,
    email character varying(255) NOT NULL,
    tenant_id character varying(64) DEFAULT '00000000-0000-0000-0000-000000000001'::character varying NOT NULL,
    kyc_status character varying(32) DEFAULT 'NONE'::character varying NOT NULL,
    kyc_tier smallint DEFAULT 0 NOT NULL,
    status character varying(32) DEFAULT 'ACTIVE'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.portal_users OWNER TO rampos;

--
-- Name: pricing_plans; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.pricing_plans (
    id character varying(64) NOT NULL,
    name character varying(255) NOT NULL,
    description text,
    currency character varying(3) DEFAULT 'USD'::character varying NOT NULL,
    period public.billing_period DEFAULT 'monthly'::public.billing_period NOT NULL,
    base_fee numeric(18,2) DEFAULT 0 NOT NULL,
    included_api_calls bigint DEFAULT 0 NOT NULL,
    included_mau bigint DEFAULT 0 NOT NULL,
    included_volume numeric(36,18) DEFAULT 0 NOT NULL,
    api_call_unit_price numeric(18,8) DEFAULT 0 NOT NULL,
    mau_unit_price numeric(18,2) DEFAULT 0 NOT NULL,
    volume_percentage_fee numeric(5,4) DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.pricing_plans OWNER TO rampos;

--
-- Name: provider_routing_policies; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.provider_routing_policies (
    id text NOT NULL,
    tenant_id text,
    provider_family text NOT NULL,
    policy_name text NOT NULL,
    corridor_code text,
    entity_type text,
    risk_tier text,
    partner_key text,
    asset_code text,
    amount_min numeric,
    amount_max numeric,
    fallback_order jsonb DEFAULT '[]'::jsonb NOT NULL,
    scorecard jsonb DEFAULT '{}'::jsonb NOT NULL,
    provider_weights jsonb DEFAULT '{}'::jsonb NOT NULL,
    lifecycle_state text DEFAULT 'active'::text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.provider_routing_policies OWNER TO rampos;

--
-- Name: rails_adapters; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.rails_adapters (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    provider_code character varying(64) NOT NULL,
    provider_name character varying(255) NOT NULL,
    adapter_type character varying(32) NOT NULL,
    config_encrypted bytea NOT NULL,
    supports_payin boolean DEFAULT true NOT NULL,
    supports_payout boolean DEFAULT true NOT NULL,
    supports_virtual_account boolean DEFAULT false NOT NULL,
    status character varying(32) DEFAULT 'ACTIVE'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT rails_status_check CHECK (((status)::text = ANY ((ARRAY['ACTIVE'::character varying, 'DISABLED'::character varying, 'TESTING'::character varying])::text[])))
);

ALTER TABLE ONLY public.rails_adapters FORCE ROW LEVEL SECURITY;


ALTER TABLE public.rails_adapters OWNER TO rampos;

--
-- Name: recon_batches; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.recon_batches (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    rails_adapter_id character varying(64),
    period_start timestamp with time zone NOT NULL,
    period_end timestamp with time zone NOT NULL,
    status character varying(32) DEFAULT 'PENDING'::character varying NOT NULL,
    total_intents integer DEFAULT 0 NOT NULL,
    matched_intents integer DEFAULT 0 NOT NULL,
    unmatched_intents integer DEFAULT 0 NOT NULL,
    discrepancy_amount numeric(30,8) DEFAULT 0,
    our_file_hash character varying(128),
    provider_file_hash character varying(128),
    report_url text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone
);

ALTER TABLE ONLY public.recon_batches FORCE ROW LEVEL SECURITY;


ALTER TABLE public.recon_batches OWNER TO rampos;

--
-- Name: refresh_tokens; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.refresh_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    token_hash bytea NOT NULL,
    user_id character varying(64) NOT NULL,
    device_info character varying(512),
    expires_at timestamp with time zone NOT NULL,
    family_id uuid NOT NULL,
    revoked boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.refresh_tokens OWNER TO rampos;

--
-- Name: registered_lp_keys; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.registered_lp_keys (
    id text NOT NULL,
    tenant_id text NOT NULL,
    lp_id text NOT NULL,
    lp_name text,
    key_hash text NOT NULL,
    can_bid_offramp boolean DEFAULT true NOT NULL,
    can_bid_onramp boolean DEFAULT true NOT NULL,
    max_bid_amount numeric,
    is_active boolean DEFAULT true NOT NULL,
    expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.registered_lp_keys OWNER TO rampos;

--
-- Name: rfq_bids; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.rfq_bids (
    id text NOT NULL,
    rfq_id text NOT NULL,
    tenant_id text NOT NULL,
    lp_id text NOT NULL,
    lp_name text,
    exchange_rate numeric NOT NULL,
    vnd_amount numeric NOT NULL,
    valid_until timestamp with time zone NOT NULL,
    state text DEFAULT 'PENDING'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT rfq_bids_state_check CHECK ((state = ANY (ARRAY['PENDING'::text, 'ACCEPTED'::text, 'REJECTED'::text, 'EXPIRED'::text])))
);


ALTER TABLE public.rfq_bids OWNER TO rampos;

--
-- Name: rfq_requests; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.rfq_requests (
    id text NOT NULL,
    tenant_id text NOT NULL,
    user_id text NOT NULL,
    direction text DEFAULT 'OFFRAMP'::text NOT NULL,
    offramp_id text,
    crypto_asset text NOT NULL,
    crypto_amount numeric NOT NULL,
    vnd_amount numeric,
    state text DEFAULT 'OPEN'::text NOT NULL,
    winning_bid_id text,
    winning_lp_id text,
    final_rate numeric,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT rfq_requests_direction_check CHECK ((direction = ANY (ARRAY['OFFRAMP'::text, 'ONRAMP'::text]))),
    CONSTRAINT rfq_requests_state_check CHECK ((state = ANY (ARRAY['OPEN'::text, 'MATCHED'::text, 'EXPIRED'::text, 'CANCELLED'::text])))
);


ALTER TABLE public.rfq_requests OWNER TO rampos;

--
-- Name: risk_score_history; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.risk_score_history (
    id uuid NOT NULL,
    user_id character varying(255) NOT NULL,
    intent_id character varying(255),
    score numeric(5,2) NOT NULL,
    triggered_rules jsonb,
    action_taken character varying(50),
    created_at timestamp with time zone DEFAULT now(),
    tenant_id character varying(64) NOT NULL,
    rule_version_id uuid,
    feature_vector jsonb,
    score_explanation jsonb,
    decision_snapshot jsonb,
    shadow_score numeric(5,2),
    shadow_decision character varying(50),
    replay_metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    CONSTRAINT risk_score_history_decision_snapshot_object CHECK (((decision_snapshot IS NULL) OR (jsonb_typeof(decision_snapshot) = 'object'::text))),
    CONSTRAINT risk_score_history_feature_vector_object CHECK (((feature_vector IS NULL) OR (jsonb_typeof(feature_vector) = 'object'::text))),
    CONSTRAINT risk_score_history_replay_metadata_object CHECK ((jsonb_typeof(replay_metadata) = 'object'::text)),
    CONSTRAINT risk_score_history_score_explanation_object CHECK (((score_explanation IS NULL) OR (jsonb_typeof(score_explanation) = 'object'::text)))
);

ALTER TABLE ONLY public.risk_score_history FORCE ROW LEVEL SECURITY;


ALTER TABLE public.risk_score_history OWNER TO rampos;

--
-- Name: sandbox_preset_scenarios; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.sandbox_preset_scenarios (
    id bigint NOT NULL,
    preset_id text NOT NULL,
    scenario_code text NOT NULL,
    sort_order smallint DEFAULT 0 NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT sandbox_preset_scenarios_code_format CHECK ((scenario_code ~ '^[A-Z0-9_]+$'::text)),
    CONSTRAINT sandbox_preset_scenarios_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text))
);


ALTER TABLE public.sandbox_preset_scenarios OWNER TO rampos;

--
-- Name: sandbox_preset_scenarios_id_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.sandbox_preset_scenarios_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.sandbox_preset_scenarios_id_seq OWNER TO rampos;

--
-- Name: sandbox_preset_scenarios_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.sandbox_preset_scenarios_id_seq OWNED BY public.sandbox_preset_scenarios.id;


--
-- Name: sandbox_presets; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.sandbox_presets (
    id text NOT NULL,
    preset_code text NOT NULL,
    name text NOT NULL,
    description text,
    seed_package_version text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    reset_strategy text DEFAULT 'RESET_TO_PRESET'::text NOT NULL,
    reset_semantics jsonb DEFAULT '{}'::jsonb NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT sandbox_presets_code_format CHECK ((preset_code ~ '^[A-Z0-9_]+$'::text)),
    CONSTRAINT sandbox_presets_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT sandbox_presets_reset_semantics_object CHECK ((jsonb_typeof(reset_semantics) = 'object'::text)),
    CONSTRAINT sandbox_presets_reset_strategy_check CHECK ((reset_strategy = ANY (ARRAY['RESET_TO_PRESET'::text, 'RESET_SCENARIO_DATA'::text, 'RESET_RUNTIME_ARTIFACTS'::text])))
);


ALTER TABLE public.sandbox_presets OWNER TO rampos;

--
-- Name: settlements; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.settlements (
    id text NOT NULL,
    offramp_intent_id text NOT NULL,
    status text DEFAULT 'PENDING'::text NOT NULL,
    bank_reference text,
    error_message text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.settlements OWNER TO rampos;

--
-- Name: smart_accounts; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.smart_accounts (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    address character varying(42) NOT NULL,
    owner_address character varying(42) NOT NULL,
    account_type character varying(64) DEFAULT 'SimpleAccount'::character varying NOT NULL,
    chain_id bigint NOT NULL,
    factory_address character varying(42),
    entry_point_address character varying(42),
    is_deployed boolean DEFAULT false NOT NULL,
    deployed_at timestamp with time zone,
    deployment_tx_hash character varying(66),
    status character varying(32) DEFAULT 'ACTIVE'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT smart_accounts_address_format CHECK (((address)::text ~ '^0x[a-fA-F0-9]{40}$'::text)),
    CONSTRAINT smart_accounts_owner_format CHECK (((owner_address)::text ~ '^0x[a-fA-F0-9]{40}$'::text)),
    CONSTRAINT smart_accounts_status_check CHECK (((status)::text = ANY ((ARRAY['ACTIVE'::character varying, 'DISABLED'::character varying, 'FROZEN'::character varying])::text[])))
);

ALTER TABLE ONLY public.smart_accounts FORCE ROW LEVEL SECURITY;


ALTER TABLE public.smart_accounts OWNER TO rampos;

--
-- Name: TABLE smart_accounts; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.smart_accounts IS 'ERC-4337 smart account registry for account ownership verification';


--
-- Name: COLUMN smart_accounts.address; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.smart_accounts.address IS 'Smart account address (counterfactual or deployed)';


--
-- Name: COLUMN smart_accounts.owner_address; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.smart_accounts.owner_address IS 'EOA address that controls this smart account';


--
-- Name: COLUMN smart_accounts.is_deployed; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.smart_accounts.is_deployed IS 'Whether the smart account has been deployed on-chain';


--
-- Name: sso_sessions; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.sso_sessions (
    id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    provider_id character varying(64) NOT NULL,
    idp_session_id character varying(255),
    access_token text,
    refresh_token text,
    id_token text,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_accessed_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.sso_sessions OWNER TO rampos;

--
-- Name: supported_tokens; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.supported_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id character varying(255) NOT NULL,
    symbol character varying(20) NOT NULL,
    name character varying(100) NOT NULL,
    decimals smallint DEFAULT 18 NOT NULL,
    logo_url text,
    website text,
    description text,
    enabled boolean DEFAULT true NOT NULL,
    min_deposit numeric(78,0) DEFAULT 0 NOT NULL,
    max_deposit numeric(78,0),
    min_withdraw numeric(78,0) DEFAULT 0 NOT NULL,
    max_withdraw numeric(78,0),
    deposit_fee_bps smallint DEFAULT 0 NOT NULL,
    withdraw_fee_bps smallint DEFAULT 10 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.supported_tokens OWNER TO rampos;

--
-- Name: TABLE supported_tokens; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.supported_tokens IS 'Stablecoin configurations per tenant';


--
-- Name: tenant_license_documents; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.tenant_license_documents (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    tenant_license_id character varying(64) NOT NULL,
    requirement_id character varying(64) NOT NULL,
    document_name character varying(255) NOT NULL,
    document_url character varying(1024) NOT NULL,
    document_hash character varying(128),
    file_size bigint,
    mime_type character varying(128),
    status character varying(32) DEFAULT 'PENDING'::character varying NOT NULL,
    reviewed_by character varying(64),
    reviewed_at timestamp with time zone,
    review_notes text,
    rejection_reason text,
    valid_from timestamp with time zone,
    valid_until timestamp with time zone,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    uploaded_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT tenant_license_documents_status_check CHECK (((status)::text = ANY ((ARRAY['PENDING'::character varying, 'APPROVED'::character varying, 'REJECTED'::character varying, 'EXPIRED'::character varying])::text[])))
);


ALTER TABLE public.tenant_license_documents OWNER TO rampos;

--
-- Name: tenant_license_status; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.tenant_license_status (
    id text NOT NULL,
    tenant_id text NOT NULL,
    requirement_id text NOT NULL,
    status text DEFAULT 'PENDING'::text NOT NULL,
    license_number text,
    issue_date timestamp with time zone,
    expiry_date timestamp with time zone,
    last_submission_id text,
    notes text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.tenant_license_status OWNER TO rampos;

--
-- Name: TABLE tenant_license_status; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.tenant_license_status IS 'Tracks each tenant''s compliance status for licensing requirements';


--
-- Name: tenant_licenses; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.tenant_licenses (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    license_type_id character varying(64) NOT NULL,
    status character varying(32) DEFAULT 'DRAFT'::character varying NOT NULL,
    license_number character varying(128),
    issued_at timestamp with time zone,
    expires_at timestamp with time zone,
    compliance_percentage numeric(5,2) DEFAULT 0 NOT NULL,
    last_compliance_check timestamp with time zone,
    submitted_at timestamp with time zone,
    reviewed_by character varying(64),
    reviewed_at timestamp with time zone,
    review_notes text,
    rejection_reason text,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT tenant_licenses_status_check CHECK (((status)::text = ANY ((ARRAY['DRAFT'::character varying, 'SUBMITTED'::character varying, 'UNDER_REVIEW'::character varying, 'APPROVED'::character varying, 'REJECTED'::character varying, 'ACTIVE'::character varying, 'EXPIRED'::character varying, 'SUSPENDED'::character varying, 'REVOKED'::character varying])::text[])))
);


ALTER TABLE public.tenant_licenses OWNER TO rampos;

--
-- Name: tenant_rate_limits; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.tenant_rate_limits (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id character varying(64) NOT NULL,
    route_group character varying(50) DEFAULT 'default'::character varying NOT NULL,
    requests_per_minute integer DEFAULT 600 NOT NULL,
    burst_limit integer DEFAULT 100 NOT NULL,
    daily_quota integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.tenant_rate_limits OWNER TO rampos;

--
-- Name: tenants; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.tenants (
    id character varying(64) NOT NULL,
    name character varying(255) NOT NULL,
    status character varying(32) DEFAULT 'ACTIVE'::character varying NOT NULL,
    api_key_hash character varying(255) NOT NULL,
    webhook_secret_hash character varying(255) NOT NULL,
    webhook_url character varying(512),
    config jsonb DEFAULT '{}'::jsonb NOT NULL,
    daily_payin_limit_vnd numeric(20,2) DEFAULT '10000000000'::bigint,
    daily_payout_limit_vnd numeric(20,2) DEFAULT '5000000000'::bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    webhook_secret_encrypted bytea,
    api_secret_encrypted bytea,
    api_secret_nonce bytea,
    webhook_secret_nonce bytea,
    api_version character varying(20) DEFAULT '2026-02-01'::character varying,
    CONSTRAINT tenants_status_check CHECK (((status)::text = ANY ((ARRAY['ACTIVE'::character varying, 'SUSPENDED'::character varying, 'PENDING'::character varying])::text[])))
);


ALTER TABLE public.tenants OWNER TO rampos;

--
-- Name: COLUMN tenants.api_key_hash; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.tenants.api_key_hash IS 'Hash of API key (Bearer token) for tenant lookup.';


--
-- Name: COLUMN tenants.webhook_secret_hash; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.tenants.webhook_secret_hash IS 'Hash of webhook secret for verification only. DO NOT use for HMAC signing.';


--
-- Name: COLUMN tenants.webhook_secret_encrypted; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.tenants.webhook_secret_encrypted IS 'Encrypted webhook secret used for HMAC signing. Encrypted using application-level encryption.';


--
-- Name: COLUMN tenants.api_secret_encrypted; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.tenants.api_secret_encrypted IS 'Encrypted API secret used for HMAC signature verification. Encrypted using application-level encryption.';


--
-- Name: COLUMN tenants.api_secret_nonce; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.tenants.api_secret_nonce IS 'AES-256-GCM nonce (12 bytes) used to encrypt api_secret_encrypted.';


--
-- Name: COLUMN tenants.webhook_secret_nonce; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON COLUMN public.tenants.webhook_secret_nonce IS 'AES-256-GCM nonce (12 bytes) used to encrypt webhook_secret_encrypted.';


--
-- Name: token_balances; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.token_balances (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id character varying(255) NOT NULL,
    user_id character varying(255) NOT NULL,
    symbol character varying(20) NOT NULL,
    chain_id bigint NOT NULL,
    balance numeric(78,0) DEFAULT 0 NOT NULL,
    pending_deposits numeric(78,0) DEFAULT 0 NOT NULL,
    pending_withdrawals numeric(78,0) DEFAULT 0 NOT NULL,
    last_synced_block bigint,
    last_sync_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.token_balances OWNER TO rampos;

--
-- Name: TABLE token_balances; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.token_balances IS 'User token balances per chain';


--
-- Name: token_chain_deployments; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.token_chain_deployments (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    token_id uuid NOT NULL,
    chain_id bigint NOT NULL,
    chain_name character varying(50) NOT NULL,
    contract_address character varying(66) NOT NULL,
    is_native boolean DEFAULT false NOT NULL,
    bridge_contract character varying(66),
    enabled boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.token_chain_deployments OWNER TO rampos;

--
-- Name: TABLE token_chain_deployments; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.token_chain_deployments IS 'Token contract addresses per chain';


--
-- Name: token_transactions; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.token_transactions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id character varying(255) NOT NULL,
    user_id character varying(255) NOT NULL,
    intent_id character varying(255),
    tx_hash character varying(66) NOT NULL,
    chain_id bigint NOT NULL,
    block_number bigint,
    symbol character varying(20) NOT NULL,
    amount numeric(78,0) NOT NULL,
    from_address character varying(66) NOT NULL,
    to_address character varying(66) NOT NULL,
    tx_type character varying(20) NOT NULL,
    status character varying(20) DEFAULT 'PENDING'::character varying NOT NULL,
    confirmations integer DEFAULT 0 NOT NULL,
    gas_used numeric(78,0),
    gas_price numeric(78,0),
    fee_amount numeric(78,0),
    fee_currency character varying(20),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    confirmed_at timestamp with time zone
);


ALTER TABLE public.token_transactions OWNER TO rampos;

--
-- Name: TABLE token_transactions; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.token_transactions IS 'On-chain token transaction history';


--
-- Name: transaction_limit_history; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.transaction_limit_history (
    id bigint NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    intent_id character varying(64),
    transaction_type character varying(32) NOT NULL,
    amount_vnd numeric(20,2) NOT NULL,
    currency character varying(16) DEFAULT 'VND'::character varying NOT NULL,
    transaction_date date DEFAULT CURRENT_DATE NOT NULL,
    transaction_month character varying(7) DEFAULT to_char(now(), 'YYYY-MM'::text) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    vietnam_date date DEFAULT ((now() AT TIME ZONE 'Asia/Ho_Chi_Minh'::text))::date NOT NULL,
    vietnam_month character varying(7) DEFAULT to_char((now() AT TIME ZONE 'Asia/Ho_Chi_Minh'::text), 'YYYY-MM'::text) NOT NULL
);


ALTER TABLE public.transaction_limit_history OWNER TO rampos;

--
-- Name: TABLE transaction_limit_history; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.transaction_limit_history IS 'Historical record of all transactions for limit calculation';


--
-- Name: transaction_limit_history_id_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.transaction_limit_history_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.transaction_limit_history_id_seq OWNER TO rampos;

--
-- Name: transaction_limit_history_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.transaction_limit_history_id_seq OWNED BY public.transaction_limit_history.id;


--
-- Name: travel_rule_disclosures; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.travel_rule_disclosures (
    id text NOT NULL,
    tenant_id text NOT NULL,
    policy_id text,
    intent_id text,
    settlement_id text,
    transaction_reference text,
    direction text DEFAULT 'OUTBOUND'::text NOT NULL,
    lifecycle_stage text DEFAULT 'PENDING'::text NOT NULL,
    asset_symbol text NOT NULL,
    asset_amount numeric DEFAULT 0 NOT NULL,
    asset_network text,
    fiat_currency text,
    fiat_amount numeric,
    originator_vasp_id text,
    beneficiary_vasp_id text,
    transport_profile text,
    disclosure_payload jsonb DEFAULT '{}'::jsonb NOT NULL,
    redaction_profile text,
    correlation_id text,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT travel_rule_disclosures_asset_amount_check CHECK ((asset_amount >= (0)::numeric)),
    CONSTRAINT travel_rule_disclosures_direction_check CHECK ((direction = ANY (ARRAY['OUTBOUND'::text, 'INBOUND'::text]))),
    CONSTRAINT travel_rule_disclosures_fiat_amount_check CHECK (((fiat_amount IS NULL) OR (fiat_amount >= (0)::numeric))),
    CONSTRAINT travel_rule_disclosures_lifecycle_stage_check CHECK ((lifecycle_stage = ANY (ARRAY['PENDING'::text, 'READY'::text, 'SENT'::text, 'ACKNOWLEDGED'::text, 'FAILED'::text, 'EXCEPTION'::text, 'WAIVED'::text]))),
    CONSTRAINT travel_rule_disclosures_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT travel_rule_disclosures_payload_object CHECK ((jsonb_typeof(disclosure_payload) = 'object'::text))
);


ALTER TABLE public.travel_rule_disclosures OWNER TO rampos;

--
-- Name: travel_rule_exception_queue; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.travel_rule_exception_queue (
    id text NOT NULL,
    tenant_id text NOT NULL,
    disclosure_id text NOT NULL,
    latest_attempt_id text,
    queue_status text DEFAULT 'OPEN'::text NOT NULL,
    severity text DEFAULT 'MEDIUM'::text NOT NULL,
    reason_code text NOT NULL,
    reason_details text,
    assigned_to text,
    due_at timestamp with time zone,
    resolved_at timestamp with time zone,
    resolution_notes text,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT travel_rule_exception_queue_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT travel_rule_exception_queue_resolution_order CHECK (((resolved_at IS NULL) OR (resolved_at >= created_at))),
    CONSTRAINT travel_rule_exception_queue_severity_check CHECK ((severity = ANY (ARRAY['LOW'::text, 'MEDIUM'::text, 'HIGH'::text, 'CRITICAL'::text]))),
    CONSTRAINT travel_rule_exception_queue_status_check CHECK ((queue_status = ANY (ARRAY['OPEN'::text, 'IN_REVIEW'::text, 'ESCALATED'::text, 'RESOLVED'::text, 'DISMISSED'::text])))
);


ALTER TABLE public.travel_rule_exception_queue OWNER TO rampos;

--
-- Name: travel_rule_policies; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.travel_rule_policies (
    id text NOT NULL,
    tenant_id text NOT NULL,
    policy_code text NOT NULL,
    display_name text NOT NULL,
    jurisdiction_code text,
    direction_scope text DEFAULT 'BOTH'::text NOT NULL,
    asset_scope jsonb DEFAULT '{}'::jsonb NOT NULL,
    threshold_amount numeric,
    threshold_currency text,
    counterparty_scope jsonb DEFAULT '{}'::jsonb NOT NULL,
    default_transport_profile text,
    default_action text DEFAULT 'REVIEW_REQUIRED'::text NOT NULL,
    policy_version text DEFAULT 'v1'::text NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT travel_rule_policies_asset_scope_object CHECK ((jsonb_typeof(asset_scope) = 'object'::text)),
    CONSTRAINT travel_rule_policies_counterparty_scope_object CHECK ((jsonb_typeof(counterparty_scope) = 'object'::text)),
    CONSTRAINT travel_rule_policies_default_action_check CHECK ((default_action = ANY (ARRAY['ALLOW'::text, 'REVIEW_REQUIRED'::text, 'DISCLOSE_BEFORE_SETTLEMENT'::text, 'DISCLOSE_AFTER_SETTLEMENT'::text, 'BLOCK'::text]))),
    CONSTRAINT travel_rule_policies_direction_scope_check CHECK ((direction_scope = ANY (ARRAY['OUTBOUND'::text, 'INBOUND'::text, 'BOTH'::text]))),
    CONSTRAINT travel_rule_policies_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT travel_rule_policies_threshold_amount_check CHECK (((threshold_amount IS NULL) OR (threshold_amount >= (0)::numeric)))
);


ALTER TABLE public.travel_rule_policies OWNER TO rampos;

--
-- Name: travel_rule_transport_attempts; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.travel_rule_transport_attempts (
    id text NOT NULL,
    tenant_id text NOT NULL,
    disclosure_id text NOT NULL,
    attempt_number integer DEFAULT 1 NOT NULL,
    transport_kind text NOT NULL,
    status text DEFAULT 'PENDING'::text NOT NULL,
    endpoint_uri text,
    request_payload jsonb DEFAULT '{}'::jsonb NOT NULL,
    response_payload jsonb,
    response_status_code integer,
    error_code text,
    error_message text,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    attempted_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    CONSTRAINT travel_rule_transport_attempts_attempt_number_check CHECK ((attempt_number >= 1)),
    CONSTRAINT travel_rule_transport_attempts_completed_after_attempted CHECK (((completed_at IS NULL) OR (completed_at >= attempted_at))),
    CONSTRAINT travel_rule_transport_attempts_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT travel_rule_transport_attempts_request_payload_object CHECK ((jsonb_typeof(request_payload) = 'object'::text)),
    CONSTRAINT travel_rule_transport_attempts_response_payload_object CHECK (((response_payload IS NULL) OR (jsonb_typeof(response_payload) = 'object'::text))),
    CONSTRAINT travel_rule_transport_attempts_status_check CHECK ((status = ANY (ARRAY['PENDING'::text, 'SENT'::text, 'ACKNOWLEDGED'::text, 'FAILED'::text, 'TIMEOUT'::text, 'REJECTED'::text])))
);


ALTER TABLE public.travel_rule_transport_attempts OWNER TO rampos;

--
-- Name: travel_rule_vasps; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.travel_rule_vasps (
    id text NOT NULL,
    tenant_id text NOT NULL,
    vasp_code text NOT NULL,
    legal_name text NOT NULL,
    display_name text,
    jurisdiction_code text,
    registration_number text,
    travel_rule_profile text,
    transport_profile text,
    endpoint_uri text,
    endpoint_public_key text,
    review_status text DEFAULT 'PENDING'::text NOT NULL,
    interoperability_status text DEFAULT 'UNKNOWN'::text NOT NULL,
    supports_inbound boolean DEFAULT false NOT NULL,
    supports_outbound boolean DEFAULT false NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT travel_rule_vasps_interop_status_check CHECK ((interoperability_status = ANY (ARRAY['UNKNOWN'::text, 'READY'::text, 'LIMITED'::text, 'DEGRADED'::text, 'DISABLED'::text]))),
    CONSTRAINT travel_rule_vasps_metadata_object CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT travel_rule_vasps_review_status_check CHECK ((review_status = ANY (ARRAY['PENDING'::text, 'APPROVED'::text, 'REJECTED'::text, 'SUSPENDED'::text])))
);


ALTER TABLE public.travel_rule_vasps OWNER TO rampos;

--
-- Name: treasury_evidence_imports; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.treasury_evidence_imports (
    id text NOT NULL,
    tenant_id text NOT NULL,
    source_family text NOT NULL,
    source_ref text NOT NULL,
    account_scope text NOT NULL,
    asset_code text NOT NULL,
    idempotency_key text NOT NULL,
    snapshot_at timestamp with time zone NOT NULL,
    available_balance numeric DEFAULT 0 NOT NULL,
    reserved_balance numeric DEFAULT 0 NOT NULL,
    source_lineage jsonb DEFAULT '{}'::jsonb NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    imported_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.treasury_evidence_imports OWNER TO rampos;

--
-- Name: usage_events; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.usage_events (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    meter_slug character varying(64) NOT NULL,
    amount numeric(36,18) NOT NULL,
    dimensions jsonb DEFAULT '{}'::jsonb,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.usage_events OWNER TO rampos;

--
-- Name: user_token_balances; Type: VIEW; Schema: public; Owner: rampos
--

CREATE VIEW public.user_token_balances AS
 SELECT tb.tenant_id,
    tb.user_id,
    tb.symbol,
    st.name AS token_name,
    st.decimals,
    st.logo_url,
    sum(tb.balance) AS total_balance,
    sum(tb.pending_deposits) AS total_pending_deposits,
    sum(tb.pending_withdrawals) AS total_pending_withdrawals,
    jsonb_agg(jsonb_build_object('chain_id', tb.chain_id, 'balance', (tb.balance)::text, 'pending_deposits', (tb.pending_deposits)::text, 'pending_withdrawals', (tb.pending_withdrawals)::text)) AS chain_balances
   FROM (public.token_balances tb
     JOIN public.supported_tokens st ON ((((st.tenant_id)::text = (tb.tenant_id)::text) AND ((st.symbol)::text = (tb.symbol)::text))))
  WHERE (st.enabled = true)
  GROUP BY tb.tenant_id, tb.user_id, tb.symbol, st.name, st.decimals, st.logo_url;


ALTER VIEW public.user_token_balances OWNER TO rampos;

--
-- Name: VIEW user_token_balances; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON VIEW public.user_token_balances IS 'Aggregated token balances view per user';


--
-- Name: user_transaction_limits; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.user_transaction_limits (
    id integer NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    tier smallint DEFAULT 0 NOT NULL,
    custom_single_limit_vnd numeric(20,2),
    custom_daily_limit_vnd numeric(20,2),
    custom_monthly_limit_vnd numeric(20,2),
    custom_manual_approval_threshold numeric(20,2),
    custom_limit_reason text,
    custom_limit_approved_by character varying(64),
    custom_limit_approved_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_transaction_limits OWNER TO rampos;

--
-- Name: TABLE user_transaction_limits; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.user_transaction_limits IS 'Tracks user-specific transaction limit overrides and tier information';


--
-- Name: user_transaction_limits_id_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.user_transaction_limits_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.user_transaction_limits_id_seq OWNER TO rampos;

--
-- Name: user_transaction_limits_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.user_transaction_limits_id_seq OWNED BY public.user_transaction_limits.id;


--
-- Name: users; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.users (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    kyc_tier smallint DEFAULT 0 NOT NULL,
    kyc_status character varying(32) DEFAULT 'PENDING'::character varying NOT NULL,
    kyc_verified_at timestamp with time zone,
    risk_score numeric(5,2) DEFAULT 0,
    risk_flags jsonb DEFAULT '[]'::jsonb,
    daily_payin_limit_vnd numeric(20,2),
    daily_payout_limit_vnd numeric(20,2),
    status character varying(32) DEFAULT 'ACTIVE'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT users_kyc_tier_check CHECK (((kyc_tier >= 0) AND (kyc_tier <= 3))),
    CONSTRAINT users_status_check CHECK (((status)::text = ANY ((ARRAY['ACTIVE'::character varying, 'SUSPENDED'::character varying, 'BLOCKED'::character varying])::text[])))
);

ALTER TABLE ONLY public.users FORCE ROW LEVEL SECURITY;


ALTER TABLE public.users OWNER TO rampos;

--
-- Name: virtual_accounts; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.virtual_accounts (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    user_id character varying(64) NOT NULL,
    rails_adapter_id character varying(64) NOT NULL,
    bank_code character varying(32) NOT NULL,
    account_number character varying(64) NOT NULL,
    account_name character varying(255) NOT NULL,
    status character varying(32) DEFAULT 'ACTIVE'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone
);

ALTER TABLE ONLY public.virtual_accounts FORCE ROW LEVEL SECURITY;


ALTER TABLE public.virtual_accounts OWNER TO rampos;

--
-- Name: vnd_limit_config; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.vnd_limit_config (
    id integer NOT NULL,
    tenant_id character varying(64) NOT NULL,
    tier_limits jsonb DEFAULT '{}'::jsonb NOT NULL,
    reset_at_vietnam_midnight boolean DEFAULT true NOT NULL,
    enforce_on_payin boolean DEFAULT true NOT NULL,
    enforce_on_payout boolean DEFAULT true NOT NULL,
    timezone character varying(64) DEFAULT 'Asia/Ho_Chi_Minh'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.vnd_limit_config OWNER TO rampos;

--
-- Name: TABLE vnd_limit_config; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.vnd_limit_config IS 'Tenant-level VND transaction limit configuration';


--
-- Name: vnd_limit_config_id_seq; Type: SEQUENCE; Schema: public; Owner: rampos
--

CREATE SEQUENCE public.vnd_limit_config_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.vnd_limit_config_id_seq OWNER TO rampos;

--
-- Name: vnd_limit_config_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: rampos
--

ALTER SEQUENCE public.vnd_limit_config_id_seq OWNED BY public.vnd_limit_config.id;


--
-- Name: webauthn_challenges; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.webauthn_challenges (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    challenge_key character varying(255) NOT NULL,
    challenge_type character varying(32) NOT NULL,
    state_json jsonb NOT NULL,
    email character varying(255),
    user_id character varying(64),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL
);


ALTER TABLE public.webauthn_challenges OWNER TO rampos;

--
-- Name: webauthn_credentials; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.webauthn_credentials (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id character varying(64) NOT NULL,
    tenant_id character varying(64) DEFAULT '00000000-0000-0000-0000-000000000001'::character varying NOT NULL,
    credential_id bytea NOT NULL,
    credential_json jsonb NOT NULL,
    name character varying(255),
    aaguid uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone,
    sign_count bigint DEFAULT 0 NOT NULL
);


ALTER TABLE public.webauthn_credentials OWNER TO rampos;

--
-- Name: webhook_configs; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.webhook_configs (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    url character varying(1024) NOT NULL,
    events jsonb DEFAULT '[]'::jsonb NOT NULL,
    active boolean DEFAULT true NOT NULL,
    secret character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.webhook_configs OWNER TO rampos;

--
-- Name: TABLE webhook_configs; Type: COMMENT; Schema: public; Owner: rampos
--

COMMENT ON TABLE public.webhook_configs IS 'Webhook endpoint configurations for tenants';


--
-- Name: webhook_events; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.webhook_events (
    id character varying(64) NOT NULL,
    tenant_id character varying(64) NOT NULL,
    event_type character varying(64) NOT NULL,
    intent_id character varying(64),
    payload jsonb NOT NULL,
    status character varying(32) DEFAULT 'PENDING'::character varying NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    max_attempts integer DEFAULT 10 NOT NULL,
    last_attempt_at timestamp with time zone,
    next_attempt_at timestamp with time zone,
    last_error text,
    delivered_at timestamp with time zone,
    response_status integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    config_id character varying(64),
    CONSTRAINT webhook_status_check CHECK (((status)::text = ANY ((ARRAY['PENDING'::character varying, 'DELIVERED'::character varying, 'FAILED'::character varying, 'CANCELLED'::character varying])::text[])))
);

ALTER TABLE ONLY public.webhook_events FORCE ROW LEVEL SECURITY;


ALTER TABLE public.webhook_events OWNER TO rampos;

--
-- Name: whitelisted_extension_actions; Type: TABLE; Schema: public; Owner: rampos
--

CREATE TABLE public.whitelisted_extension_actions (
    action_id text NOT NULL,
    label text NOT NULL,
    description text NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    approval_required boolean DEFAULT true NOT NULL,
    rollout_scope jsonb DEFAULT '{}'::jsonb NOT NULL,
    source text DEFAULT 'registry_seed'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.whitelisted_extension_actions OWNER TO rampos;

--
-- Name: account_balances id; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.account_balances ALTER COLUMN id SET DEFAULT nextval('public.account_balances_id_seq'::regclass);


--
-- Name: audit_log id; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.audit_log ALTER COLUMN id SET DEFAULT nextval('public.audit_log_id_seq'::regclass);


--
-- Name: bank_webhook_secrets id; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.bank_webhook_secrets ALTER COLUMN id SET DEFAULT nextval('public.bank_webhook_secrets_id_seq'::regclass);


--
-- Name: compliance_audit_log sequence_number; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.compliance_audit_log ALTER COLUMN sequence_number SET DEFAULT nextval('public.compliance_audit_log_sequence_number_seq'::regclass);


--
-- Name: ledger_entries sequence; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.ledger_entries ALTER COLUMN sequence SET DEFAULT nextval('public.ledger_entries_sequence_seq'::regclass);


--
-- Name: sandbox_preset_scenarios id; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.sandbox_preset_scenarios ALTER COLUMN id SET DEFAULT nextval('public.sandbox_preset_scenarios_id_seq'::regclass);


--
-- Name: transaction_limit_history id; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.transaction_limit_history ALTER COLUMN id SET DEFAULT nextval('public.transaction_limit_history_id_seq'::regclass);


--
-- Name: user_transaction_limits id; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.user_transaction_limits ALTER COLUMN id SET DEFAULT nextval('public.user_transaction_limits_id_seq'::regclass);


--
-- Name: vnd_limit_config id; Type: DEFAULT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.vnd_limit_config ALTER COLUMN id SET DEFAULT nextval('public.vnd_limit_config_id_seq'::regclass);


--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
1	initial schema	2026-03-13 04:58:37.906582+00	t	\\x571e32b6a3b0fe385dd9f513ce1727a33c989d8e9588431efb8136ff278b6afebb9842eedc803eddc23c126b250946c0	448219586
2	seed data	2026-03-13 04:58:38.358248+00	t	\\x141af681f8e88429450fb2f0b837ecb4f327d89c347c72bff756af96a9a6b556ee54b90a9d31cef79d8695e55a2d7efe	498734299
3	rule versions	2026-03-13 04:58:38.859857+00	t	\\x2d613852b1e224e6e9fb791efef91036e8a16a14f61b11018e6852cae322a3cde585ed979a657c6321b090e127fbf7d4	37325425
4	score history	2026-03-13 04:58:38.911948+00	t	\\xc81c71ef81d503a1ef85c33a38c8bc0f5b6736c22bcc2839a3e9d267b1ec1ea15737f63c222b25b1b05921b3a0614030	27086747
5	case notes	2026-03-13 04:58:38.94154+00	t	\\xc8b29dcc96116b9ab384eac3272468d3258f05cf48a83374b5382b03096f2d24e87c6c77b684fb1d42486b39dfb1c256	17235111
6	enable rls	2026-03-13 04:58:38.961387+00	t	\\x4d53ffd9780cc76dd8b52cb62bdfc9ecf42d5b708a15feaa46b037f869dac1ccddfd237aab2472bf638526b798c40655	8476585
7	compliance transactions	2026-03-13 04:58:38.972388+00	t	\\x8476e2e4a8eea3a6742c18a2ef2ece5814387c9ad6aecca25fddab1a6b305ce958337340ac8318af52cd7c12679381fe	22336686
8	add missing rls	2026-03-13 04:58:38.997334+00	t	\\xc6a86c87303c544bd106322e298883327bcab7ee4fe54eaaef6716178d9face03a9a0fb2949943e3eaccdc7e10427927	41693424
9	add webhook secret	2026-03-13 04:58:39.041746+00	t	\\x8d61a6dced99120e88402a4ac5ace7057125c2705194f9b85768a6a1b6c58b87e2ac2e3ad478c02e745b606ad3544355	3952725
10	smart accounts	2026-03-13 04:58:39.048102+00	t	\\x9584f3c05a36f8607ab32e48e83a940b78d66772affc8cfd473dc72f8a80ad122751ce9cb184d481ac5d6d098aa82291	52559252
11	add api secret	2026-03-13 04:58:39.103313+00	t	\\x6727f2c1da74757e45c634b4ca740ba18e9f2d4466c3954191f8756139ea2fe14a3a110a9ef968a2205e8895791bdd2b	3855738
12	bank confirmations	2026-03-13 04:58:39.109682+00	t	\\x8772d705d8a3d8dfa0a08eb760079f327b6909e5e14cdf167094b2937fd62522ee4d758dc44632f7b72822233a6a77c3	98122195
13	compliance integrity	2026-03-13 04:58:39.21047+00	t	\\xde526d64f35fa304561fd13b2406dd68a537ef7e0ec5b1d46d7eb2dab3c2e6f6536182cd249929fe2f645d0f6d64619c	16015126
14	rls fail closed	2026-03-13 04:58:39.229047+00	t	\\xfa76a8d4ea62a788a53a83173c2740dc08150c035e8ec9cbb194fbb352af98817756e0620bf8c6e4518ef6bf7dfcc67c	20326775
15	license management	2026-03-13 04:58:39.252108+00	t	\\x877b72fe628309dcb6611359b96f0e349ace9ecca465d7d7eab502a695ca13b2f931fd62ef59fe43ed81fd50bdbd5f5c	163392161
16	multi stablecoin	2026-03-13 04:58:39.418138+00	t	\\x116dd2498c589142c76a9754a88628c18c407872058b99264fc622cc8fd4660f8f03941424dd9b25c5f5cbe6ff972811	136913028
17	custom domains	2026-03-13 04:58:39.557724+00	t	\\xc0bb7803f055c74e63fab670a2a1fb7a2fc1deaa7a5d42fa90d95142300a23107eece18ff38f20ac9b91c4a57c511311	38229917
18	enterprise sso	2026-03-13 04:58:39.598475+00	t	\\x3ccd8fa02638a854a705e7e5aa2f89ba7572c5c8ce01d91f7fe7b33e7acfea1420aab6f0b05b4b5fc2743faaaa379d5b	71646508
19	usage billing	2026-03-13 04:58:39.672724+00	t	\\xacaede9028d12a49e4fdd3d40fe64f05f6a528c4a972f655395d223c28bbb92b211233798c7f13f083d7451caafa8db4	100200702
20	compliance audit trail	2026-03-13 04:58:39.775657+00	t	\\xb03787a57a9ce99292088aaf6f0f8b566232f0877eb0d8b3072df60192762d48fdc94ce1c4e7cf894eddab41d0d3b145	58124499
21	vnd transaction limits	2026-03-13 04:58:39.836328+00	t	\\xdb6a967eb9c6820e7ec843d31127937670007e08667061dee4c119c02b117b9261d9ac044cdc5a0a997f29d55706dd47	102327011
22	licensing requirements	2026-03-13 04:58:39.941397+00	t	\\x258be42b4e77d755515253bfbc310c3545af635c2645104c2db6e82cb08d99623a02acc969734c808a20239040de7a1b	77608460
23	encrypt secrets nonce	2026-03-13 04:58:40.021528+00	t	\\x0c6d492ff6d315d34cc450438202efe29298e4884f9630967e9f73334c225bf21d0cbf805e0be86704b5c93a1a641b84	3951327
24	webauthn credentials	2026-03-13 04:58:40.028103+00	t	\\xc23e4c2343da814eda02deb546f1756b3b3998bdb2fd608ddcd201c833108945d61a14a7832c70274eb9e32b8f697461	94868546
25	portal kyc cases	2026-03-13 04:58:40.125619+00	t	\\x659f99600071f9a8d76af65d37155893a8d8fb041cb291d0587a94977e7db70cdc062e2065de319d9f2b4d38b614861c	65403825
26	webhook configs	2026-03-13 04:58:40.193572+00	t	\\x64bfcf239975957ae1cffa7f650e4a3cd54b89c445b55d47fe2582882dcb84dce510c96376679767f1de27236e9db0db	40005574
27	offramp intents	2026-03-13 04:58:40.2361+00	t	\\xfa86920bebf404a3626eb25a0eb2e51061fdec2d9d6a50c63e30df3d595fd926eaaea2dfa0f29cd6b4517f5c5e5f4f76	36154528
28	magic link tokens	2026-03-13 04:58:40.27482+00	t	\\x34a4d55f70010d22f5d630d48f742f19e545865ae8cf6105b7456806609c64c7b0f4ea94dcc4c810a151d806ebfa283d	33357563
29	refresh tokens	2026-03-13 04:58:40.310709+00	t	\\x4825337a170d67c25536b5e97b856b5b7565805cd515c2e3c8b99856fc5cbf1ece1a83b5b0f809471cdd2c4a7da8a67b	39365215
30	tenant rate limits	2026-03-13 04:58:40.352706+00	t	\\x598d36edf7fac21ac11c3bf893f04616c202b477b2dec1db7749df12d1d1cb152a3fd30500467721c82e53b8eb8f9942	22485079
31	tenant api version	2026-03-13 04:58:40.377664+00	t	\\x82236cf13f0424559567c96550331efe5069dff43d8631cd173fcd0acc7a34314b6038b727bca055622fd637f662e4c2	4229279
32	settlements	2026-03-13 04:58:40.385359+00	t	\\xd17891c59c0c74c39f80659d6d4ef234eb204bbbe422aea970978dd32e0b3d0c338d1e6c290a3eff6390db2213b353e4	26853108
33	rfq auction	2026-03-13 04:58:40.414788+00	t	\\x18a2f1393fc6f9eaab0a23c6f1bbe649551bce9c146ca119c7aa95a7ebc368342bf8523aba33df678bec643df602bb54	65403508
34	lp keys	2026-03-13 04:58:40.482773+00	t	\\xeebd5aedaca7d1fd1fba5c7bb7c43a898a958313af9791e0bbba94c5897f5ae0bfacf393519c9a146ff8d4b9a8ff4c0b	41013467
35	sandbox presets	2026-03-13 04:58:40.526358+00	t	\\xb6d61709889f079b2057b249a93b6ebfe4025d10888c641eb8102f3d53e8bf5e49c3f96dc557a9f79d1c8dc3b507ca9d	62972337
36	lp reliability snapshots	2026-03-13 04:58:40.59221+00	t	\\x7d47d2acb7562d1ac6c1836583e6c0fe20e3c1589fc9270dce17d261734121ad98b4c9d4a07186eb96d6af4764d19e3e	53786955
37	travel rule	2026-03-13 04:58:40.648493+00	t	\\x1fcd4ff3647bbdb57ddc026a62154717597ced47ef94ac3e934f9a90f97763aed07803a1758d081238d8e04fe83bc631	208694867
38	risk lab replay metadata	2026-03-13 04:58:40.859889+00	t	\\xed3d43a43da40165a13c4ebaec1b9da70ba2600d67df21515090e4452d8a850462e38dedd763831937eeb4cc105b99c6	25393299
39	rescreening runs	2026-03-13 04:58:40.887803+00	t	\\xb9e4ee346b122c379aea6aaae9c4815898816d0611c236b2a0ad49b98241389c3b130077c3648cc7c5036cf390279418	44076713
40	kyc passport	2026-03-13 04:58:40.934351+00	t	\\xaed99fff937ec1b324673fe506ba04033737d6a3d1667e7dc9163985de0316c5f9600c9d4599e744b0a51a5556b7601e	53214766
41	kyb graph	2026-03-13 04:58:40.991333+00	t	\\x21261aff98fef76c6e3d58d82e6f75538fc609c57c13ac00fec9a4b63e009bfa01f58efffa8bd3cf79135e1d720cc84a	39876972
42	config bundle governance	2026-03-13 04:58:41.033705+00	t	\\xe8039ccb1ee2b752ee5ec9f8019fadd0dcca635620457d41604412027108e6479b16abfdbd69e187c9599952cee38d22	47942308
43	partner registry	2026-03-13 04:58:41.08417+00	t	\\x22ac7f3d6ca64eeca117c8247cb738cb1b0d95de5a7447cbf1a6e256a2d444f7f01a43d78ecc84bec8c95cd922000e37	126334328
44	corridor packs	2026-03-13 04:58:41.213082+00	t	\\x955c81933b6bb49f09290d7d9c8d77344c2197426a1949e24990a6ee603df0fb1b6ba39e2e981420c8c4a577e7e57c7c	175939875
45	payment method capabilities	2026-03-13 04:58:41.391647+00	t	\\x3a74395df8d6f1c65e33e3be4258a7ddb6db802daf6deff36a396bcf6c7f3a03fe092672343075dc8223a85cb660e956	33555007
46	provider routing	2026-03-13 04:58:41.427678+00	t	\\xb023a4b8afe39c9289e9c994fa3bb5364ac28631cc4a56e02fe9197549bdda251ea44bb8385be49adcff0eef9e64d819	30351177
47	kyb evidence packages	2026-03-13 04:58:41.460372+00	t	\\xcad4ec9f3d556bf2dce6e8148d95a5c0d12bd63536c54032d3b4b72b07f3ad727b5bb49320b1fca9ee5c39df69536e63	74846336
48	treasury evidence imports	2026-03-13 04:58:41.537931+00	t	\\x2825a6efa928071448879aa1da395c990dedb717d2109ffc05b1e9302380761df0dda0c0f22b4b511849baac83ce95e8	33608185
999	seed data	2026-03-13 04:58:41.574137+00	t	\\xb0584dfe88b3157ffe64fc6cabc124f1ab4d22c8404de2657fd2bd24275523c020b4ac337511afc6d7ec324ba164ea8f	13658049
\.


--
-- Data for Name: account_balances; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.account_balances (id, tenant_id, user_id, account_type, currency, balance, last_entry_id, last_sequence, updated_at) FROM stdin;
1	tenant_a_123	\N	ASSET_BANK_VCB	VND	10000000.00000000	\N	\N	2026-03-13 04:58:38.358248+00
2	tenant_a_123	user_a_1	LIABILITY_USER_MAIN	VND	8000000.00000000	\N	\N	2026-03-13 04:58:38.358248+00
3	tenant_a_123		ASSET_BANK_VCB	VND	58000000.00000000	\N	\N	2026-03-13 04:58:38.358248+00
5	tenant_a_123	user_a_5	LIABILITY_USER_MAIN	VND	25000000.00000000	\N	\N	2026-03-13 04:58:41.574137+00
6	tenant_a_123	user_a_5	LIABILITY_USER_MAIN	USDT	1000.00000000	\N	\N	2026-03-13 04:58:41.574137+00
\.


--
-- Data for Name: aml_cases; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.aml_cases (id, tenant_id, user_id, intent_id, case_type, severity, status, rule_id, rule_name, detection_data, assigned_to, resolution, resolved_at, created_at, updated_at) FROM stdin;
aml_case_1	tenant_a_123	user_a_4	\N	SANCTIONS	CRITICAL	OPEN	\N	\N	{"list": "OFAC", "match_score": 0.98}	\N	\N	\N	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
aml_case_2	tenant_a_123	user_a_1	intent_payin_001	VELOCITY	MEDIUM	REVIEW	\N	\N	{"count": 5, "window": "24h"}	\N	\N	\N	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
\.


--
-- Data for Name: aml_rule_versions; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.aml_rule_versions (id, tenant_id, version_number, rules_json, is_active, created_at, created_by, activated_at, version_state, version_label, parent_version_id, version_metadata, scorer_config, decision_thresholds) FROM stdin;
\.


--
-- Data for Name: audit_log; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.audit_log (id, tenant_id, actor_type, actor_id, action, resource_type, resource_id, details, ip_address, user_agent, request_id, created_at, prev_hash, entry_hash) FROM stdin;
\.


--
-- Data for Name: bank_confirmations; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.bank_confirmations (id, tenant_id, provider, reference_code, bank_reference, bank_tx_id, amount, currency, sender_account, sender_name, receiver_account, receiver_name, status, matched_intent_id, matched_at, webhook_received_at, webhook_signature, webhook_signature_verified, raw_payload, processing_notes, transaction_time, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: bank_webhook_secrets; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.bank_webhook_secrets (id, tenant_id, provider, secret_encrypted, algorithm, header_name, is_active, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: billing_meters; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.billing_meters (id, slug, name, type, aggregation, created_at) FROM stdin;
meter_01	api_requests	API Requests	api_calls	sum	2026-03-13 04:58:39.672724+00
meter_02	tx_volume_usd	Transaction Volume (USD)	transaction_volume	sum	2026-03-13 04:58:39.672724+00
meter_03	active_users	Monthly Active Users	active_users	unique_count	2026-03-13 04:58:39.672724+00
\.


--
-- Data for Name: case_notes; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.case_notes (id, case_id, author_id, content, note_type, is_internal, created_at, tenant_id) FROM stdin;
\.


--
-- Data for Name: compliance_audit_log; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.compliance_audit_log (id, tenant_id, event_type, actor_id, actor_type, action_details, resource_type, resource_id, sequence_number, previous_hash, current_hash, ip_address, user_agent, request_id, created_at) FROM stdin;
\.


--
-- Data for Name: compliance_rescreening_runs; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.compliance_rescreening_runs (id, tenant_id, user_id, trigger_kind, status, priority, restriction_status, alert_codes, details, scheduled_for, executed_at, next_run_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: compliance_transactions; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.compliance_transactions (id, tenant_id, user_id, intent_id, transaction_type, amount_vnd, created_at) FROM stdin;
\.


--
-- Data for Name: config_bundle_exports; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.config_bundle_exports (id, tenant_id, tenant_name, action_mode, sections, payload, approval_status, rollout_scope, provenance, is_active, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: corridor_compliance_hooks; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.corridor_compliance_hooks (id, corridor_pack_id, hook_kind, provider_key, required, config, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: corridor_cutoff_policies; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.corridor_cutoff_policies (id, corridor_pack_id, timezone, cutoff_windows, holiday_calendar, retry_rule, exception_policy, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: corridor_eligibility_rules; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.corridor_eligibility_rules (id, corridor_pack_id, partner_id, entity_type, method_family, amount_bounds, compliance_requirements, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: corridor_fee_profiles; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.corridor_fee_profiles (id, corridor_pack_id, fee_currency, base_fee, fx_spread_bps, liquidity_cost_bps, surcharge_bps, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: corridor_pack_endpoints; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.corridor_pack_endpoints (id, corridor_pack_id, endpoint_role, partner_id, provider_key, adapter_key, entity_type, rail, method_family, settlement_mode, instrument_family, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: corridor_packs; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.corridor_packs (id, tenant_id, corridor_code, source_market, destination_market, source_currency, destination_currency, settlement_direction, fee_model, lifecycle_state, rollout_state, eligibility_state, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: corridor_rollout_scopes; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.corridor_rollout_scopes (id, corridor_pack_id, tenant_id, environment, geography, method_family, rollout_state, approval_reference, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: credential_references; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.credential_references (id, partner_id, credential_kind, locator, environment, approval_reference, rotation_metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: custom_domains; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.custom_domains (id, tenant_id, domain, status, dns_verification_token, dns_verification_record, ssl_certificate, health_check_path, last_health_check, is_primary, custom_headers, redirects, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: daily_usage; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.daily_usage (tenant_id, meter_slug, date, total_amount) FROM stdin;
\.


--
-- Data for Name: identity_providers; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.identity_providers (id, tenant_id, name, slug, type, protocol, is_enabled, config, role_mappings, default_role, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: intents; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.intents (id, tenant_id, user_id, intent_type, state, state_history, amount, currency, actual_amount, rails_provider, reference_code, bank_tx_id, chain_id, tx_hash, from_address, to_address, metadata, idempotency_key, created_at, updated_at, expires_at, completed_at) FROM stdin;
intent_payin_001	tenant_a_123	user_a_1	PAYIN_VND	COMPLETED	[]	10000000.00000000	VND	10000000.00000000	VCB_DIRECT	\N	\N	\N	\N	\N	\N	{}	\N	2026-03-11 04:58:38.358248+00	2026-03-11 05:03:38.358248+00	\N	2026-03-11 05:03:38.358248+00
intent_payout_001	tenant_a_123	user_a_1	PAYOUT_VND	COMPLETED	[]	2000000.00000000	VND	2000000.00000000	VCB_DIRECT	\N	\N	\N	\N	\N	\N	{}	\N	2026-03-12 04:58:38.358248+00	2026-03-12 05:08:38.358248+00	\N	2026-03-12 05:08:38.358248+00
intent_pending_1	tenant_a_123	user_a_2	PAYIN_VND	PROCESSING	[]	500000.00000000	VND	\N	VCB_DIRECT	\N	\N	\N	\N	\N	\N	{}	\N	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00	\N	\N
intent_pending_2	tenant_b_456	user_b_1	PAYOUT_VND	INITIATED	[]	100000.00000000	VND	\N	VN_PAY	\N	\N	\N	\N	\N	\N	{}	\N	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00	\N	\N
intent_payin_a5_1	tenant_a_123	user_a_5	PAYIN_VND	COMPLETED	[]	50000000.00000000	VND	50000000.00000000	VCB_DIRECT	\N	\N	\N	\N	\N	\N	{}	\N	2026-03-13 01:58:41.574137+00	2026-03-13 01:58:41.574137+00	\N	2026-03-13 01:58:41.574137+00
intent_trade_a5_1	tenant_a_123	user_a_5	TRADE_EXECUTED	COMPLETED	[]	25000000.00000000	VND	25000000.00000000	\N	\N	\N	\N	\N	\N	\N	{"rate": 25000, "buy_amount": 1000, "buy_currency": "USDT"}	\N	2026-03-13 02:58:41.574137+00	2026-03-13 02:58:41.574137+00	\N	2026-03-13 02:58:41.574137+00
intent_exp_001	tenant_a_123	user_a_6	PAYIN_VND	EXPIRED	[]	200000.00000000	VND	\N	\N	\N	\N	\N	\N	\N	\N	{}	\N	2026-03-10 04:58:41.574137+00	2026-03-13 04:58:41.574137+00	2026-03-11 04:58:41.574137+00	\N
intent_can_001	tenant_b_456	user_b_4	PAYOUT_VND	CANCELLED	[]	500000.00000000	VND	\N	\N	\N	\N	\N	\N	\N	\N	{}	\N	2026-03-12 04:58:41.574137+00	2026-03-13 04:58:41.574137+00	\N	\N
\.


--
-- Data for Name: invoices; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.invoices (id, tenant_id, period_start, period_end, status, currency, subtotal, tax, total, line_items, due_date, paid_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyb_entities; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyb_entities (id, tenant_id, entity_type, display_name, jurisdiction, status, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyb_evidence_packages; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyb_evidence_packages (id, tenant_id, institution_entity_id, institution_legal_name, provider_family, provider_policy_id, corridor_code, review_status, review_notes, export_status, export_artifact_uri, metadata, exported_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyb_evidence_sources; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyb_evidence_sources (id, package_id, source_kind, source_ref, document_id, collected_at, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyb_ownership_edges; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyb_ownership_edges (id, tenant_id, source_id, target_id, edge_type, ownership_pct, effective_from, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyb_ubo_evidence_links; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyb_ubo_evidence_links (id, package_id, owner_entity_id, ownership_pct, evidence_source_ref, review_state, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyc_passport_acceptance_policies; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyc_passport_acceptance_policies (id, tenant_id, min_tier, max_age_days, allowed_source_tenants, requires_manual_review, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyc_passport_consent_grants; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyc_passport_consent_grants (id, tenant_id, passport_id, target_tenant_id, consent_status, scope, granted_at, revoked_at, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyc_passport_vault; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyc_passport_vault (id, tenant_id, user_id, source_tenant_id, status, kyc_tier, fields_shared, verified_at, expires_at, revoked_at, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: kyc_records; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.kyc_records (id, tenant_id, user_id, tier, provider, provider_reference, status, verification_data, rejection_reason, documents, submitted_at, verified_at, expires_at) FROM stdin;
\.


--
-- Data for Name: ledger_entries; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, sequence, description, metadata, created_at) FROM stdin;
ledger_pi_1	tenant_a_123	\N	intent_payin_001	tx_pi_1	ASSET_BANK_VCB	DEBIT	10000000.00000000	VND	10000000.00000000	1	\N	{}	2026-03-11 05:03:38.358248+00
ledger_pi_2	tenant_a_123	user_a_1	intent_payin_001	tx_pi_1	LIABILITY_USER_MAIN	CREDIT	10000000.00000000	VND	10000000.00000000	2	\N	{}	2026-03-11 05:03:38.358248+00
ledger_po_1	tenant_a_123	user_a_1	intent_payout_001	tx_po_1	LIABILITY_USER_MAIN	DEBIT	2000000.00000000	VND	8000000.00000000	3	\N	{}	2026-03-12 05:08:38.358248+00
ledger_po_2	tenant_a_123	\N	intent_payout_001	tx_po_1	ASSET_BANK_VCB	CREDIT	2000000.00000000	VND	8000000.00000000	4	\N	{}	2026-03-12 05:08:38.358248+00
ledger_pi_a5_1	tenant_a_123	\N	intent_payin_a5_1	tx_pi_a5_1	ASSET_BANK_VCB	DEBIT	50000000.00000000	VND	50000000.00000000	5	\N	{}	2026-03-13 01:58:41.574137+00
ledger_pi_a5_2	tenant_a_123	user_a_5	intent_payin_a5_1	tx_pi_a5_1	LIABILITY_USER_MAIN	CREDIT	50000000.00000000	VND	50000000.00000000	6	\N	{}	2026-03-13 01:58:41.574137+00
ledger_tr_a5_1	tenant_a_123	user_a_5	intent_trade_a5_1	tx_tr_a5_1	LIABILITY_USER_MAIN	DEBIT	25000000.00000000	VND	25000000.00000000	7	\N	{}	2026-03-13 02:58:41.574137+00
ledger_tr_a5_2	tenant_a_123	user_a_5	intent_trade_a5_1	tx_tr_a5_1	LIABILITY_USER_MAIN	CREDIT	1000.00000000	USDT	1000.00000000	8	\N	{}	2026-03-13 02:58:41.574137+00
\.


--
-- Data for Name: license_requirements; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.license_requirements (id, license_type_id, requirement_name, requirement_code, description, is_mandatory, document_type, validation_rules, display_order, created_at, updated_at) FROM stdin;
lr_ex_001	lt_exchange	Business Registration Certificate	BRC	Valid business registration certificate from the Ministry of Planning and Investment	t	CERTIFICATE	{}	1	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_ex_002	lt_exchange	Capital Proof	CAPITAL_PROOF	Proof of minimum capital requirement (100 billion VND)	t	FINANCIAL	{}	2	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_ex_003	lt_exchange	AML/CFT Policy	AML_POLICY	Anti-money laundering and counter-terrorism financing policy document	t	POLICY	{}	3	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_ex_004	lt_exchange	Security Audit Report	SECURITY_AUDIT	Third-party security audit report for trading platform	t	AUDIT	{}	4	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_ex_005	lt_exchange	Insurance Certificate	INSURANCE	Insurance coverage for digital asset custody	f	CERTIFICATE	{}	5	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_cu_001	lt_custodial	Business Registration Certificate	BRC	Valid business registration certificate	t	CERTIFICATE	{}	1	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_cu_002	lt_custodial	Cold Storage Policy	COLD_STORAGE	Documentation of cold storage procedures and security measures	t	POLICY	{}	2	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_cu_003	lt_custodial	Key Management Procedure	KEY_MGMT	Cryptographic key management procedures	t	POLICY	{}	3	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_cu_004	lt_custodial	SOC 2 Type II Report	SOC2	SOC 2 Type II compliance report	t	AUDIT	{}	4	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_cu_005	lt_custodial	Disaster Recovery Plan	DR_PLAN	Business continuity and disaster recovery plan	t	POLICY	{}	5	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_pm_001	lt_payment	Business Registration Certificate	BRC	Valid business registration certificate	t	CERTIFICATE	{}	1	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_pm_002	lt_payment	PCI DSS Compliance	PCI_DSS	Payment Card Industry Data Security Standard compliance certificate	t	CERTIFICATE	{}	2	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_pm_003	lt_payment	Bank Partnership Agreement	BANK_AGREEMENT	Partnership agreement with a licensed bank	t	CONTRACT	{}	3	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lr_pm_004	lt_payment	Transaction Monitoring System	TXN_MONITORING	Documentation of transaction monitoring capabilities	t	POLICY	{}	4	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
\.


--
-- Data for Name: license_submissions; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.license_submissions (id, tenant_id, requirement_id, documents, status, submitted_by, submitted_at, reviewed_at, reviewer_notes) FROM stdin;
\.


--
-- Data for Name: license_types; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.license_types (id, name, code, description, jurisdiction, regulatory_body, is_active, metadata, created_at, updated_at) FROM stdin;
lt_exchange	Crypto Exchange License	EXCHANGE	License to operate a cryptocurrency exchange platform in Vietnam	VN	State Bank of Vietnam	t	{}	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lt_custodial	Digital Asset Custody License	CUSTODIAL	License to provide custodial services for digital assets	VN	State Bank of Vietnam	t	{}	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
lt_payment	Payment Service Provider License	PAYMENT	License to provide payment processing services	VN	State Bank of Vietnam	t	{}	2026-03-13 04:58:39.252108+00	2026-03-13 04:58:39.252108+00
\.


--
-- Data for Name: lp_reliability_snapshots; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.lp_reliability_snapshots (id, tenant_id, lp_id, direction, window_kind, window_started_at, window_ended_at, snapshot_version, quote_count, fill_count, reject_count, settlement_count, dispute_count, fill_rate, reject_rate, dispute_rate, avg_slippage_bps, p95_settlement_latency_seconds, reliability_score, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: magic_link_tokens; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.magic_link_tokens (id, token_hash, email, expires_at, used, created_at) FROM stdin;
\.


--
-- Data for Name: offramp_intents; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.offramp_intents (id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate, locked_rate_id, fees, net_vnd_amount, gross_vnd_amount, bank_account, deposit_address, tx_hash, bank_reference, state, state_history, created_at, updated_at, quote_expires_at) FROM stdin;
\.


--
-- Data for Name: partner_approval_references; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.partner_approval_references (id, tenant_id, action_class, status, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: partner_capabilities; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.partner_capabilities (id, partner_id, capability_family, environment, adapter_key, provider_key, supported_rails, supported_methods, approval_status, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: partner_health_signals; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.partner_health_signals (id, partner_capability_id, status, source, score, incident_summary, evidence, observed_at, created_at) FROM stdin;
\.


--
-- Data for Name: partner_rollout_scopes; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.partner_rollout_scopes (id, partner_capability_id, tenant_id, environment, corridor_code, geography, method_family, rollout_state, rollback_target, approval_reference, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: partners; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.partners (id, tenant_id, partner_class, code, display_name, legal_name, market, jurisdiction, service_domain, lifecycle_state, approval_status, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: payment_method_capabilities; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.payment_method_capabilities (id, corridor_pack_id, partner_capability_id, method_family, funding_source, settlement_direction, presentment_model, card_funding_enabled, policy_flags, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: portal_kyc_cases; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.portal_kyc_cases (id, user_id, tenant_id, status, tier, full_name, date_of_birth, document_type, document_number, address, reviewer_notes, reviewed_at, submitted_at, updated_at) FROM stdin;
\.


--
-- Data for Name: portal_kyc_documents; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.portal_kyc_documents (id, case_id, user_id, document_type, filename, content_type, file_size, file_path, uploaded_at) FROM stdin;
\.


--
-- Data for Name: portal_users; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.portal_users (id, email, tenant_id, kyc_status, kyc_tier, status, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: pricing_plans; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.pricing_plans (id, name, description, currency, period, base_fee, included_api_calls, included_mau, included_volume, api_call_unit_price, mau_unit_price, volume_percentage_fee, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: provider_routing_policies; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.provider_routing_policies (id, tenant_id, provider_family, policy_name, corridor_code, entity_type, risk_tier, partner_key, asset_code, amount_min, amount_max, fallback_order, scorecard, provider_weights, lifecycle_state, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: rails_adapters; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.rails_adapters (id, tenant_id, provider_code, provider_name, adapter_type, config_encrypted, supports_payin, supports_payout, supports_virtual_account, status, created_at, updated_at) FROM stdin;
rails_a_bank	tenant_a_123	VCB_DIRECT	Vietcombank Direct	BANK	\\xdeadbeef	t	t	t	ACTIVE	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
rails_a_crypto	tenant_a_123	FIREBLOCKS	Fireblocks	CRYPTO	\\xcafebabe	t	t	f	ACTIVE	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
rails_b_psp	tenant_b_456	VN_PAY	VNPay	PSP	\\xbadf00dd	t	f	t	ACTIVE	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
\.


--
-- Data for Name: recon_batches; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.recon_batches (id, tenant_id, rails_adapter_id, period_start, period_end, status, total_intents, matched_intents, unmatched_intents, discrepancy_amount, our_file_hash, provider_file_hash, report_url, created_at, completed_at) FROM stdin;
\.


--
-- Data for Name: refresh_tokens; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.refresh_tokens (id, token_hash, user_id, device_info, expires_at, family_id, revoked, created_at) FROM stdin;
\.


--
-- Data for Name: registered_lp_keys; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.registered_lp_keys (id, tenant_id, lp_id, lp_name, key_hash, can_bid_offramp, can_bid_onramp, max_bid_amount, is_active, expires_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: rfq_bids; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.rfq_bids (id, rfq_id, tenant_id, lp_id, lp_name, exchange_rate, vnd_amount, valid_until, state, created_at) FROM stdin;
\.


--
-- Data for Name: rfq_requests; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.rfq_requests (id, tenant_id, user_id, direction, offramp_id, crypto_asset, crypto_amount, vnd_amount, state, winning_bid_id, winning_lp_id, final_rate, expires_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: risk_score_history; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.risk_score_history (id, user_id, intent_id, score, triggered_rules, action_taken, created_at, tenant_id, rule_version_id, feature_vector, score_explanation, decision_snapshot, shadow_score, shadow_decision, replay_metadata) FROM stdin;
\.


--
-- Data for Name: sandbox_preset_scenarios; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.sandbox_preset_scenarios (id, preset_id, scenario_code, sort_order, metadata, created_at) FROM stdin;
1	sandbox_preset_baseline	PAYIN_BASELINE	10	{"flow": "payin", "mode": "happy_path"}	2026-03-13 04:58:40.526358+00
2	sandbox_preset_baseline	OFFRAMP_BASELINE	20	{"flow": "offramp", "mode": "happy_path"}	2026-03-13 04:58:40.526358+00
3	sandbox_preset_baseline	WEBHOOK_RETRY_BASELINE	30	{"flow": "webhook", "mode": "retry"}	2026-03-13 04:58:40.526358+00
4	sandbox_preset_payin_failure	PAYIN_BANK_TIMEOUT	10	{"flow": "payin", "mode": "failure_drill"}	2026-03-13 04:58:40.526358+00
5	sandbox_preset_payin_failure	PAYIN_COMPLIANCE_REVIEW	20	{"flow": "payin", "mode": "manual_review"}	2026-03-13 04:58:40.526358+00
6	sandbox_preset_liquidity_drill	RFQ_BASELINE	10	{"flow": "rfq", "mode": "auction"}	2026-03-13 04:58:40.526358+00
7	sandbox_preset_liquidity_drill	LP_NO_FILL	20	{"flow": "rfq", "mode": "failure_drill"}	2026-03-13 04:58:40.526358+00
8	sandbox_preset_liquidity_drill	SETTLEMENT_DELAY	30	{"flow": "settlement", "mode": "recovery"}	2026-03-13 04:58:40.526358+00
\.


--
-- Data for Name: sandbox_presets; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.sandbox_presets (id, preset_code, name, description, seed_package_version, metadata, reset_strategy, reset_semantics, is_active, created_at, updated_at) FROM stdin;
sandbox_preset_baseline	BASELINE	Baseline Sandbox	Balanced preset for common pay-in, payout, RFQ, and webhook replay coverage.	2026-03-08	{"category": "general", "supports_replay": true, "operator_surface": "admin"}	RESET_TO_PRESET	{"drop_seeded_users": true, "drop_runtime_events": true, "drop_seeded_intents": true, "drop_seeded_balances": true, "preserve_admin_credentials": true}	t	2026-03-13 04:58:40.526358+00	2026-03-13 04:58:40.526358+00
sandbox_preset_payin_failure	PAYIN_FAILURE_DRILL	Pay-in Failure Drill	Preset for deterministic payment failure drills and post-failure replay export.	2026-03-08	{"category": "payin", "supports_replay": true, "operator_surface": "admin"}	RESET_SCENARIO_DATA	{"drop_runtime_events": true, "drop_seeded_intents": true, "drop_seeded_webhooks": true, "preserve_seeded_users": true, "preserve_admin_credentials": true}	t	2026-03-13 04:58:40.526358+00	2026-03-13 04:58:40.526358+00
sandbox_preset_liquidity_drill	LIQUIDITY_DRILL	Liquidity Drill	Preset for RFQ, no-fill, and delayed-settlement liquidity exercises.	2026-03-08	{"category": "liquidity", "supports_replay": true, "operator_surface": "admin"}	RESET_RUNTIME_ARTIFACTS	{"drop_rfq_artifacts": true, "drop_runtime_events": true, "preserve_seeded_users": true, "drop_settlement_attempts": true, "preserve_admin_credentials": true}	t	2026-03-13 04:58:40.526358+00	2026-03-13 04:58:40.526358+00
\.


--
-- Data for Name: settlements; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.settlements (id, offramp_intent_id, status, bank_reference, error_message, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: smart_accounts; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.smart_accounts (id, tenant_id, user_id, address, owner_address, account_type, chain_id, factory_address, entry_point_address, is_deployed, deployed_at, deployment_tx_hash, status, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: sso_sessions; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.sso_sessions (id, user_id, provider_id, idp_session_id, access_token, refresh_token, id_token, expires_at, created_at, last_accessed_at) FROM stdin;
\.


--
-- Data for Name: supported_tokens; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.supported_tokens (id, tenant_id, symbol, name, decimals, logo_url, website, description, enabled, min_deposit, max_deposit, min_withdraw, max_withdraw, deposit_fee_bps, withdraw_fee_bps, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: tenant_license_documents; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.tenant_license_documents (id, tenant_id, tenant_license_id, requirement_id, document_name, document_url, document_hash, file_size, mime_type, status, reviewed_by, reviewed_at, review_notes, rejection_reason, valid_from, valid_until, metadata, uploaded_at, updated_at) FROM stdin;
\.


--
-- Data for Name: tenant_license_status; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.tenant_license_status (id, tenant_id, requirement_id, status, license_number, issue_date, expiry_date, last_submission_id, notes, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: tenant_licenses; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.tenant_licenses (id, tenant_id, license_type_id, status, license_number, issued_at, expires_at, compliance_percentage, last_compliance_check, submitted_at, reviewed_by, reviewed_at, review_notes, rejection_reason, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: tenant_rate_limits; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.tenant_rate_limits (id, tenant_id, route_group, requests_per_minute, burst_limit, daily_quota, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: tenants; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.tenants (id, name, status, api_key_hash, webhook_secret_hash, webhook_url, config, daily_payin_limit_vnd, daily_payout_limit_vnd, created_at, updated_at, webhook_secret_encrypted, api_secret_encrypted, api_secret_nonce, webhook_secret_nonce, api_version) FROM stdin;
tenant_a_123	CryptoExchange A	ACTIVE	$2a$10$VpbFu6t08ba2YgvuHE.NTeFb69pD3PaFy/Er3TT85Vl9mcdiJJzKi	$2a$10$..gM6hK.13py4S5XRU8jz.KpXbNAuXuWqsJaF38e6g5ZpTQFjXfOS	https://api.exchange-a.com/webhooks/rampos	{"tier": "enterprise", "features": ["payin", "payout", "va"]}	50000000000.00	20000000000.00	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00	\N	\N	\N	\N	2026-02-01
tenant_b_456	WalletApp B	ACTIVE	$2a$10$NmfKsl2ka2khODsxsigQ/.WAyNF5LHuJgQo0.jHfji5BrYm9P2ZuO	$2a$10$ZHZDVFZm/kTyYDjFwID88Oge2wJFAdWvbVMYCx5hA7AG9Lj1FhaiO	https://backend.wallet-b.app/callbacks	{"tier": "standard", "features": ["payin"]}	10000000000.00	5000000000.00	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00	\N	\N	\N	\N	2026-02-01
tenant_c_789	Startup C	PENDING	$2a$10$qKF8tH16VQyxGZS4itkQ9uZW4vlqnadKc7Dww5Gx.YJMN2WZZM6mi	$2a$10$5ywe09Qy8abxZOpeKfnbluRM3CQqBJCrErvCoihSb03Gr1wPIk0E6	https://api.startup-c.com/webhooks	{"tier": "starter"}	1000000000.00	500000000.00	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:41.574137+00	\N	\N	\N	\N	2026-02-01
tenant_stage_local	Local Stage Tenant	ACTIVE	044146db5ff225f7ace4b9afb3d9318aa99e8bc26121c463a7410da450e37575	seeded	\N	{"tier": "enterprise"}	10000000000.00	5000000000.00	2026-03-13 05:08:34.123762+00	2026-03-13 05:08:34.123762+00	\N	\\x6c6f63616c5f73746167655f6170695f736563726574	\N	\N	2026-02-01
\.


--
-- Data for Name: token_balances; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.token_balances (id, tenant_id, user_id, symbol, chain_id, balance, pending_deposits, pending_withdrawals, last_synced_block, last_sync_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: token_chain_deployments; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.token_chain_deployments (id, token_id, chain_id, chain_name, contract_address, is_native, bridge_contract, enabled, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: token_transactions; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.token_transactions (id, tenant_id, user_id, intent_id, tx_hash, chain_id, block_number, symbol, amount, from_address, to_address, tx_type, status, confirmations, gas_used, gas_price, fee_amount, fee_currency, created_at, updated_at, confirmed_at) FROM stdin;
\.


--
-- Data for Name: transaction_limit_history; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.transaction_limit_history (id, tenant_id, user_id, intent_id, transaction_type, amount_vnd, currency, transaction_date, transaction_month, created_at, vietnam_date, vietnam_month) FROM stdin;
\.


--
-- Data for Name: travel_rule_disclosures; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.travel_rule_disclosures (id, tenant_id, policy_id, intent_id, settlement_id, transaction_reference, direction, lifecycle_stage, asset_symbol, asset_amount, asset_network, fiat_currency, fiat_amount, originator_vasp_id, beneficiary_vasp_id, transport_profile, disclosure_payload, redaction_profile, correlation_id, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: travel_rule_exception_queue; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.travel_rule_exception_queue (id, tenant_id, disclosure_id, latest_attempt_id, queue_status, severity, reason_code, reason_details, assigned_to, due_at, resolved_at, resolution_notes, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: travel_rule_policies; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.travel_rule_policies (id, tenant_id, policy_code, display_name, jurisdiction_code, direction_scope, asset_scope, threshold_amount, threshold_currency, counterparty_scope, default_transport_profile, default_action, policy_version, is_active, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: travel_rule_transport_attempts; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.travel_rule_transport_attempts (id, tenant_id, disclosure_id, attempt_number, transport_kind, status, endpoint_uri, request_payload, response_payload, response_status_code, error_code, error_message, metadata, attempted_at, completed_at) FROM stdin;
\.


--
-- Data for Name: travel_rule_vasps; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.travel_rule_vasps (id, tenant_id, vasp_code, legal_name, display_name, jurisdiction_code, registration_number, travel_rule_profile, transport_profile, endpoint_uri, endpoint_public_key, review_status, interoperability_status, supports_inbound, supports_outbound, metadata, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: treasury_evidence_imports; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.treasury_evidence_imports (id, tenant_id, source_family, source_ref, account_scope, asset_code, idempotency_key, snapshot_at, available_balance, reserved_balance, source_lineage, metadata, imported_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: usage_events; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.usage_events (id, tenant_id, meter_slug, amount, dimensions, "timestamp") FROM stdin;
\.


--
-- Data for Name: user_transaction_limits; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.user_transaction_limits (id, tenant_id, user_id, tier, custom_single_limit_vnd, custom_daily_limit_vnd, custom_monthly_limit_vnd, custom_manual_approval_threshold, custom_limit_reason, custom_limit_approved_by, custom_limit_approved_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.users (id, tenant_id, kyc_tier, kyc_status, kyc_verified_at, risk_score, risk_flags, daily_payin_limit_vnd, daily_payout_limit_vnd, status, created_at, updated_at) FROM stdin;
user_a_1	tenant_a_123	2	VERIFIED	\N	10.50	[]	\N	\N	ACTIVE	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
user_a_2	tenant_a_123	1	VERIFIED	\N	5.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
user_a_3	tenant_a_123	0	PENDING	\N	0.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
user_a_4	tenant_a_123	3	VERIFIED	\N	85.00	[]	\N	\N	SUSPENDED	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
user_b_1	tenant_b_456	1	VERIFIED	\N	2.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
user_b_2	tenant_b_456	0	PENDING	\N	0.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
user_b_3	tenant_b_456	1	REJECTED	\N	95.00	[]	\N	\N	BLOCKED	2026-03-13 04:58:38.358248+00	2026-03-13 04:58:38.358248+00
user_a_5	tenant_a_123	1	VERIFIED	\N	12.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_a_6	tenant_a_123	2	VERIFIED	\N	3.50	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_a_7	tenant_a_123	3	VERIFIED	\N	0.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_a_8	tenant_a_123	1	PENDING	\N	0.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_b_4	tenant_b_456	1	VERIFIED	\N	1.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_b_5	tenant_b_456	0	PENDING	\N	0.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_b_6	tenant_b_456	3	VERIFIED	\N	0.50	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_c_1	tenant_c_789	3	VERIFIED	\N	0.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_c_2	tenant_c_789	2	VERIFIED	\N	5.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_c_3	tenant_c_789	1	VERIFIED	\N	15.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_c_4	tenant_c_789	0	PENDING	\N	0.00	[]	\N	\N	ACTIVE	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
user_c_5	tenant_c_789	0	REJECTED	\N	80.00	[]	\N	\N	BLOCKED	2026-03-13 04:58:41.574137+00	2026-03-13 04:58:41.574137+00
\.


--
-- Data for Name: virtual_accounts; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.virtual_accounts (id, tenant_id, user_id, rails_adapter_id, bank_code, account_number, account_name, status, created_at, expires_at) FROM stdin;
va_a_1	tenant_a_123	user_a_1	rails_a_bank	VCB	999123456789	RAMP USER A1	ACTIVE	2026-03-13 04:58:38.358248+00	\N
va_a_2	tenant_a_123	user_a_2	rails_a_bank	VCB	999987654321	RAMP USER A2	ACTIVE	2026-03-13 04:58:38.358248+00	\N
va_b_1	tenant_b_456	user_b_1	rails_b_psp	BIDV	888111222333	WALLET USER B1	ACTIVE	2026-03-13 04:58:38.358248+00	\N
\.


--
-- Data for Name: vnd_limit_config; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.vnd_limit_config (id, tenant_id, tier_limits, reset_at_vietnam_midnight, enforce_on_payin, enforce_on_payout, timezone, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: webauthn_challenges; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.webauthn_challenges (id, challenge_key, challenge_type, state_json, email, user_id, created_at, expires_at) FROM stdin;
\.


--
-- Data for Name: webauthn_credentials; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.webauthn_credentials (id, user_id, tenant_id, credential_id, credential_json, name, aaguid, created_at, last_used_at, sign_count) FROM stdin;
\.


--
-- Data for Name: webhook_configs; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.webhook_configs (id, tenant_id, url, events, active, secret, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: webhook_events; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.webhook_events (id, tenant_id, event_type, intent_id, payload, status, attempts, max_attempts, last_attempt_at, next_attempt_at, last_error, delivered_at, response_status, created_at, config_id) FROM stdin;
wh_evt_1	tenant_a_123	intent.completed	intent_payin_a5_1	{"id": "intent_payin_a5_1", "type": "PAYIN_VND", "amount": 50000000, "status": "COMPLETED"}	DELIVERED	0	10	\N	\N	\N	2026-03-13 01:58:41.574137+00	200	2026-03-13 01:58:41.574137+00	\N
wh_evt_2	tenant_a_123	intent.failed	intent_exp_001	{"id": "intent_exp_001", "type": "PAYIN_VND", "status": "EXPIRED"}	FAILED	0	10	\N	\N	\N	\N	500	2026-03-11 04:58:41.574137+00	\N
wh_evt_3	tenant_b_456	intent.cancelled	intent_can_001	{"id": "intent_can_001", "type": "PAYOUT_VND", "status": "CANCELLED"}	PENDING	0	10	\N	\N	\N	\N	\N	2026-03-13 04:57:41.574137+00	\N
\.


--
-- Data for Name: whitelisted_extension_actions; Type: TABLE DATA; Schema: public; Owner: rampos
--

COPY public.whitelisted_extension_actions (action_id, label, description, enabled, approval_required, rollout_scope, source, created_at, updated_at) FROM stdin;
branding.apply	Apply branding bundle	Imports approved branding fields from a config bundle.	t	t	{"scope": "tenant"}	registry_seed	2026-03-13 04:58:41.033705+00	2026-03-13 04:58:41.033705+00
domains.attach	Attach domain bundle	Imports approved custom-domain configuration.	t	t	{"scope": "tenant"}	registry_seed	2026-03-13 04:58:41.033705+00	2026-03-13 04:58:41.033705+00
webhooks.sync	Sync webhook preferences	Imports approved webhook event selections only.	t	t	{"scope": "tenant"}	registry_seed	2026-03-13 04:58:41.033705+00	2026-03-13 04:58:41.033705+00
\.


--
-- Name: account_balances_id_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.account_balances_id_seq', 6, true);


--
-- Name: audit_log_id_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.audit_log_id_seq', 1, false);


--
-- Name: bank_webhook_secrets_id_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.bank_webhook_secrets_id_seq', 1, false);


--
-- Name: compliance_audit_log_sequence_number_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.compliance_audit_log_sequence_number_seq', 1, false);


--
-- Name: ledger_entries_sequence_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.ledger_entries_sequence_seq', 8, true);


--
-- Name: sandbox_preset_scenarios_id_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.sandbox_preset_scenarios_id_seq', 8, true);


--
-- Name: transaction_limit_history_id_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.transaction_limit_history_id_seq', 1, false);


--
-- Name: user_transaction_limits_id_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.user_transaction_limits_id_seq', 1, false);


--
-- Name: vnd_limit_config_id_seq; Type: SEQUENCE SET; Schema: public; Owner: rampos
--

SELECT pg_catalog.setval('public.vnd_limit_config_id_seq', 1, false);


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: account_balances account_balances_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.account_balances
    ADD CONSTRAINT account_balances_pkey PRIMARY KEY (id);


--
-- Name: account_balances account_balances_unique; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.account_balances
    ADD CONSTRAINT account_balances_unique UNIQUE (tenant_id, user_id, account_type, currency);


--
-- Name: aml_cases aml_cases_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.aml_cases
    ADD CONSTRAINT aml_cases_pkey PRIMARY KEY (id);


--
-- Name: aml_rule_versions aml_rule_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.aml_rule_versions
    ADD CONSTRAINT aml_rule_versions_pkey PRIMARY KEY (id);


--
-- Name: aml_rule_versions aml_rule_versions_tenant_id_version_number_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.aml_rule_versions
    ADD CONSTRAINT aml_rule_versions_tenant_id_version_number_key UNIQUE (tenant_id, version_number);


--
-- Name: audit_log audit_log_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.audit_log
    ADD CONSTRAINT audit_log_pkey PRIMARY KEY (id);


--
-- Name: bank_confirmations bank_confirmations_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.bank_confirmations
    ADD CONSTRAINT bank_confirmations_pkey PRIMARY KEY (id);


--
-- Name: bank_webhook_secrets bank_webhook_secrets_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.bank_webhook_secrets
    ADD CONSTRAINT bank_webhook_secrets_pkey PRIMARY KEY (id);


--
-- Name: bank_webhook_secrets bank_webhook_secrets_tenant_id_provider_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.bank_webhook_secrets
    ADD CONSTRAINT bank_webhook_secrets_tenant_id_provider_key UNIQUE (tenant_id, provider);


--
-- Name: billing_meters billing_meters_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.billing_meters
    ADD CONSTRAINT billing_meters_pkey PRIMARY KEY (id);


--
-- Name: billing_meters billing_meters_slug_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.billing_meters
    ADD CONSTRAINT billing_meters_slug_key UNIQUE (slug);


--
-- Name: case_notes case_notes_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.case_notes
    ADD CONSTRAINT case_notes_pkey PRIMARY KEY (id);


--
-- Name: compliance_audit_log compliance_audit_log_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.compliance_audit_log
    ADD CONSTRAINT compliance_audit_log_pkey PRIMARY KEY (id);


--
-- Name: compliance_rescreening_runs compliance_rescreening_runs_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.compliance_rescreening_runs
    ADD CONSTRAINT compliance_rescreening_runs_pkey PRIMARY KEY (id);


--
-- Name: compliance_transactions compliance_transactions_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.compliance_transactions
    ADD CONSTRAINT compliance_transactions_pkey PRIMARY KEY (id);


--
-- Name: config_bundle_exports config_bundle_exports_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.config_bundle_exports
    ADD CONSTRAINT config_bundle_exports_pkey PRIMARY KEY (id);


--
-- Name: corridor_compliance_hooks corridor_compliance_hooks_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_compliance_hooks
    ADD CONSTRAINT corridor_compliance_hooks_pkey PRIMARY KEY (id);


--
-- Name: corridor_cutoff_policies corridor_cutoff_policies_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_cutoff_policies
    ADD CONSTRAINT corridor_cutoff_policies_pkey PRIMARY KEY (id);


--
-- Name: corridor_eligibility_rules corridor_eligibility_rules_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_eligibility_rules
    ADD CONSTRAINT corridor_eligibility_rules_pkey PRIMARY KEY (id);


--
-- Name: corridor_fee_profiles corridor_fee_profiles_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_fee_profiles
    ADD CONSTRAINT corridor_fee_profiles_pkey PRIMARY KEY (id);


--
-- Name: corridor_pack_endpoints corridor_pack_endpoints_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_pack_endpoints
    ADD CONSTRAINT corridor_pack_endpoints_pkey PRIMARY KEY (id);


--
-- Name: corridor_packs corridor_packs_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_packs
    ADD CONSTRAINT corridor_packs_pkey PRIMARY KEY (id);


--
-- Name: corridor_packs corridor_packs_unique_code; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_packs
    ADD CONSTRAINT corridor_packs_unique_code UNIQUE (tenant_id, corridor_code);


--
-- Name: corridor_rollout_scopes corridor_rollout_scopes_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_rollout_scopes
    ADD CONSTRAINT corridor_rollout_scopes_pkey PRIMARY KEY (id);


--
-- Name: credential_references credential_references_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.credential_references
    ADD CONSTRAINT credential_references_pkey PRIMARY KEY (id);


--
-- Name: custom_domains custom_domains_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.custom_domains
    ADD CONSTRAINT custom_domains_pkey PRIMARY KEY (id);


--
-- Name: daily_usage daily_usage_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.daily_usage
    ADD CONSTRAINT daily_usage_pkey PRIMARY KEY (tenant_id, meter_slug, date);


--
-- Name: identity_providers identity_providers_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.identity_providers
    ADD CONSTRAINT identity_providers_pkey PRIMARY KEY (id);


--
-- Name: intents intents_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.intents
    ADD CONSTRAINT intents_pkey PRIMARY KEY (id);


--
-- Name: invoices invoices_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_pkey PRIMARY KEY (id);


--
-- Name: kyb_entities kyb_entities_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_entities
    ADD CONSTRAINT kyb_entities_pkey PRIMARY KEY (id);


--
-- Name: kyb_evidence_packages kyb_evidence_packages_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_evidence_packages
    ADD CONSTRAINT kyb_evidence_packages_pkey PRIMARY KEY (id);


--
-- Name: kyb_evidence_sources kyb_evidence_sources_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_evidence_sources
    ADD CONSTRAINT kyb_evidence_sources_pkey PRIMARY KEY (id);


--
-- Name: kyb_ownership_edges kyb_ownership_edges_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_ownership_edges
    ADD CONSTRAINT kyb_ownership_edges_pkey PRIMARY KEY (id);


--
-- Name: kyb_ubo_evidence_links kyb_ubo_evidence_links_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_ubo_evidence_links
    ADD CONSTRAINT kyb_ubo_evidence_links_pkey PRIMARY KEY (id);


--
-- Name: kyc_passport_acceptance_policies kyc_passport_acceptance_policies_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyc_passport_acceptance_policies
    ADD CONSTRAINT kyc_passport_acceptance_policies_pkey PRIMARY KEY (id);


--
-- Name: kyc_passport_consent_grants kyc_passport_consent_grants_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyc_passport_consent_grants
    ADD CONSTRAINT kyc_passport_consent_grants_pkey PRIMARY KEY (id);


--
-- Name: kyc_passport_vault kyc_passport_vault_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyc_passport_vault
    ADD CONSTRAINT kyc_passport_vault_pkey PRIMARY KEY (id);


--
-- Name: kyc_records kyc_records_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyc_records
    ADD CONSTRAINT kyc_records_pkey PRIMARY KEY (id);


--
-- Name: ledger_entries ledger_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.ledger_entries
    ADD CONSTRAINT ledger_entries_pkey PRIMARY KEY (id);


--
-- Name: license_requirements license_requirements_license_type_id_requirement_code_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.license_requirements
    ADD CONSTRAINT license_requirements_license_type_id_requirement_code_key UNIQUE (license_type_id, requirement_code);


--
-- Name: license_requirements license_requirements_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.license_requirements
    ADD CONSTRAINT license_requirements_pkey PRIMARY KEY (id);


--
-- Name: license_submissions license_submissions_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.license_submissions
    ADD CONSTRAINT license_submissions_pkey PRIMARY KEY (id);


--
-- Name: license_types license_types_code_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.license_types
    ADD CONSTRAINT license_types_code_key UNIQUE (code);


--
-- Name: license_types license_types_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.license_types
    ADD CONSTRAINT license_types_pkey PRIMARY KEY (id);


--
-- Name: lp_reliability_snapshots lp_reliability_snapshots_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.lp_reliability_snapshots
    ADD CONSTRAINT lp_reliability_snapshots_pkey PRIMARY KEY (id);


--
-- Name: lp_reliability_snapshots lp_reliability_snapshots_unique_window; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.lp_reliability_snapshots
    ADD CONSTRAINT lp_reliability_snapshots_unique_window UNIQUE (tenant_id, lp_id, direction, window_kind, window_started_at, window_ended_at, snapshot_version);


--
-- Name: magic_link_tokens magic_link_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.magic_link_tokens
    ADD CONSTRAINT magic_link_tokens_pkey PRIMARY KEY (id);


--
-- Name: offramp_intents offramp_intents_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.offramp_intents
    ADD CONSTRAINT offramp_intents_pkey PRIMARY KEY (id);


--
-- Name: partner_approval_references partner_approval_references_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_approval_references
    ADD CONSTRAINT partner_approval_references_pkey PRIMARY KEY (id);


--
-- Name: partner_capabilities partner_capabilities_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_capabilities
    ADD CONSTRAINT partner_capabilities_pkey PRIMARY KEY (id);


--
-- Name: partner_health_signals partner_health_signals_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_health_signals
    ADD CONSTRAINT partner_health_signals_pkey PRIMARY KEY (id);


--
-- Name: partner_rollout_scopes partner_rollout_scopes_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_rollout_scopes
    ADD CONSTRAINT partner_rollout_scopes_pkey PRIMARY KEY (id);


--
-- Name: partners partners_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partners
    ADD CONSTRAINT partners_pkey PRIMARY KEY (id);


--
-- Name: payment_method_capabilities payment_method_capabilities_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.payment_method_capabilities
    ADD CONSTRAINT payment_method_capabilities_pkey PRIMARY KEY (id);


--
-- Name: portal_kyc_cases portal_kyc_cases_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.portal_kyc_cases
    ADD CONSTRAINT portal_kyc_cases_pkey PRIMARY KEY (id);


--
-- Name: portal_kyc_documents portal_kyc_documents_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.portal_kyc_documents
    ADD CONSTRAINT portal_kyc_documents_pkey PRIMARY KEY (id);


--
-- Name: portal_users portal_users_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.portal_users
    ADD CONSTRAINT portal_users_pkey PRIMARY KEY (id);


--
-- Name: pricing_plans pricing_plans_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.pricing_plans
    ADD CONSTRAINT pricing_plans_pkey PRIMARY KEY (id);


--
-- Name: provider_routing_policies provider_routing_policies_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.provider_routing_policies
    ADD CONSTRAINT provider_routing_policies_pkey PRIMARY KEY (id);


--
-- Name: rails_adapters rails_adapters_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rails_adapters
    ADD CONSTRAINT rails_adapters_pkey PRIMARY KEY (id);


--
-- Name: rails_adapters rails_adapters_tenant_id_provider_code_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rails_adapters
    ADD CONSTRAINT rails_adapters_tenant_id_provider_code_key UNIQUE (tenant_id, provider_code);


--
-- Name: recon_batches recon_batches_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.recon_batches
    ADD CONSTRAINT recon_batches_pkey PRIMARY KEY (id);


--
-- Name: refresh_tokens refresh_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_pkey PRIMARY KEY (id);


--
-- Name: registered_lp_keys registered_lp_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.registered_lp_keys
    ADD CONSTRAINT registered_lp_keys_pkey PRIMARY KEY (id);


--
-- Name: registered_lp_keys registered_lp_keys_tenant_id_key_hash_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.registered_lp_keys
    ADD CONSTRAINT registered_lp_keys_tenant_id_key_hash_key UNIQUE (tenant_id, key_hash);


--
-- Name: registered_lp_keys registered_lp_keys_tenant_id_lp_id_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.registered_lp_keys
    ADD CONSTRAINT registered_lp_keys_tenant_id_lp_id_key UNIQUE (tenant_id, lp_id);


--
-- Name: rfq_bids rfq_bids_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rfq_bids
    ADD CONSTRAINT rfq_bids_pkey PRIMARY KEY (id);


--
-- Name: rfq_requests rfq_requests_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rfq_requests
    ADD CONSTRAINT rfq_requests_pkey PRIMARY KEY (id);


--
-- Name: risk_score_history risk_score_history_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.risk_score_history
    ADD CONSTRAINT risk_score_history_pkey PRIMARY KEY (id);


--
-- Name: sandbox_preset_scenarios sandbox_preset_scenarios_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.sandbox_preset_scenarios
    ADD CONSTRAINT sandbox_preset_scenarios_pkey PRIMARY KEY (id);


--
-- Name: sandbox_preset_scenarios sandbox_preset_scenarios_unique; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.sandbox_preset_scenarios
    ADD CONSTRAINT sandbox_preset_scenarios_unique UNIQUE (preset_id, scenario_code);


--
-- Name: sandbox_presets sandbox_presets_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.sandbox_presets
    ADD CONSTRAINT sandbox_presets_pkey PRIMARY KEY (id);


--
-- Name: sandbox_presets sandbox_presets_preset_code_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.sandbox_presets
    ADD CONSTRAINT sandbox_presets_preset_code_key UNIQUE (preset_code);


--
-- Name: settlements settlements_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.settlements
    ADD CONSTRAINT settlements_pkey PRIMARY KEY (id);


--
-- Name: smart_accounts smart_accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.smart_accounts
    ADD CONSTRAINT smart_accounts_pkey PRIMARY KEY (id);


--
-- Name: sso_sessions sso_sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.sso_sessions
    ADD CONSTRAINT sso_sessions_pkey PRIMARY KEY (id);


--
-- Name: supported_tokens supported_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.supported_tokens
    ADD CONSTRAINT supported_tokens_pkey PRIMARY KEY (id);


--
-- Name: tenant_license_documents tenant_license_documents_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_license_documents
    ADD CONSTRAINT tenant_license_documents_pkey PRIMARY KEY (id);


--
-- Name: tenant_license_status tenant_license_status_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_license_status
    ADD CONSTRAINT tenant_license_status_pkey PRIMARY KEY (id);


--
-- Name: tenant_license_status tenant_license_status_tenant_id_requirement_id_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_license_status
    ADD CONSTRAINT tenant_license_status_tenant_id_requirement_id_key UNIQUE (tenant_id, requirement_id);


--
-- Name: tenant_licenses tenant_licenses_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_licenses
    ADD CONSTRAINT tenant_licenses_pkey PRIMARY KEY (id);


--
-- Name: tenant_licenses tenant_licenses_tenant_id_license_type_id_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_licenses
    ADD CONSTRAINT tenant_licenses_tenant_id_license_type_id_key UNIQUE (tenant_id, license_type_id);


--
-- Name: tenant_rate_limits tenant_rate_limits_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_rate_limits
    ADD CONSTRAINT tenant_rate_limits_pkey PRIMARY KEY (id);


--
-- Name: tenant_rate_limits tenant_rate_limits_tenant_id_route_group_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_rate_limits
    ADD CONSTRAINT tenant_rate_limits_tenant_id_route_group_key UNIQUE (tenant_id, route_group);


--
-- Name: tenants tenants_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenants
    ADD CONSTRAINT tenants_pkey PRIMARY KEY (id);


--
-- Name: token_balances token_balances_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_balances
    ADD CONSTRAINT token_balances_pkey PRIMARY KEY (id);


--
-- Name: token_chain_deployments token_chain_deployments_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_chain_deployments
    ADD CONSTRAINT token_chain_deployments_pkey PRIMARY KEY (id);


--
-- Name: token_transactions token_transactions_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_transactions
    ADD CONSTRAINT token_transactions_pkey PRIMARY KEY (id);


--
-- Name: transaction_limit_history transaction_limit_history_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.transaction_limit_history
    ADD CONSTRAINT transaction_limit_history_pkey PRIMARY KEY (id);


--
-- Name: travel_rule_disclosures travel_rule_disclosures_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_disclosures
    ADD CONSTRAINT travel_rule_disclosures_pkey PRIMARY KEY (id);


--
-- Name: travel_rule_exception_queue travel_rule_exception_queue_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_exception_queue
    ADD CONSTRAINT travel_rule_exception_queue_pkey PRIMARY KEY (id);


--
-- Name: travel_rule_policies travel_rule_policies_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_policies
    ADD CONSTRAINT travel_rule_policies_pkey PRIMARY KEY (id);


--
-- Name: travel_rule_policies travel_rule_policies_unique_code_version; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_policies
    ADD CONSTRAINT travel_rule_policies_unique_code_version UNIQUE (tenant_id, policy_code, policy_version);


--
-- Name: travel_rule_transport_attempts travel_rule_transport_attempts_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_transport_attempts
    ADD CONSTRAINT travel_rule_transport_attempts_pkey PRIMARY KEY (id);


--
-- Name: travel_rule_transport_attempts travel_rule_transport_attempts_unique_attempt; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_transport_attempts
    ADD CONSTRAINT travel_rule_transport_attempts_unique_attempt UNIQUE (disclosure_id, attempt_number);


--
-- Name: travel_rule_vasps travel_rule_vasps_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_vasps
    ADD CONSTRAINT travel_rule_vasps_pkey PRIMARY KEY (id);


--
-- Name: travel_rule_vasps travel_rule_vasps_unique_code; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_vasps
    ADD CONSTRAINT travel_rule_vasps_unique_code UNIQUE (tenant_id, vasp_code);


--
-- Name: treasury_evidence_imports treasury_evidence_imports_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.treasury_evidence_imports
    ADD CONSTRAINT treasury_evidence_imports_pkey PRIMARY KEY (id);


--
-- Name: custom_domains uq_domain; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.custom_domains
    ADD CONSTRAINT uq_domain UNIQUE (domain);


--
-- Name: identity_providers uq_idp_slug; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.identity_providers
    ADD CONSTRAINT uq_idp_slug UNIQUE (slug);


--
-- Name: identity_providers uq_idp_tenant_slug; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.identity_providers
    ADD CONSTRAINT uq_idp_tenant_slug UNIQUE (tenant_id, slug);


--
-- Name: portal_users uq_portal_users_email_tenant; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.portal_users
    ADD CONSTRAINT uq_portal_users_email_tenant UNIQUE (email, tenant_id);


--
-- Name: supported_tokens uq_tenant_token; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.supported_tokens
    ADD CONSTRAINT uq_tenant_token UNIQUE (tenant_id, symbol);


--
-- Name: token_chain_deployments uq_token_chain; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_chain_deployments
    ADD CONSTRAINT uq_token_chain UNIQUE (token_id, chain_id);


--
-- Name: token_balances uq_user_token_chain; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_balances
    ADD CONSTRAINT uq_user_token_chain UNIQUE (tenant_id, user_id, symbol, chain_id);


--
-- Name: webauthn_challenges uq_webauthn_challenge_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webauthn_challenges
    ADD CONSTRAINT uq_webauthn_challenge_key UNIQUE (challenge_key);


--
-- Name: webauthn_credentials uq_webauthn_credential_id; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webauthn_credentials
    ADD CONSTRAINT uq_webauthn_credential_id UNIQUE (credential_id);


--
-- Name: usage_events usage_events_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.usage_events
    ADD CONSTRAINT usage_events_pkey PRIMARY KEY (id);


--
-- Name: user_transaction_limits user_limits_unique; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.user_transaction_limits
    ADD CONSTRAINT user_limits_unique UNIQUE (tenant_id, user_id);


--
-- Name: user_transaction_limits user_transaction_limits_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.user_transaction_limits
    ADD CONSTRAINT user_transaction_limits_pkey PRIMARY KEY (id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (tenant_id, id);


--
-- Name: virtual_accounts virtual_accounts_bank_code_account_number_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.virtual_accounts
    ADD CONSTRAINT virtual_accounts_bank_code_account_number_key UNIQUE (bank_code, account_number);


--
-- Name: virtual_accounts virtual_accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.virtual_accounts
    ADD CONSTRAINT virtual_accounts_pkey PRIMARY KEY (id);


--
-- Name: vnd_limit_config vnd_limit_config_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.vnd_limit_config
    ADD CONSTRAINT vnd_limit_config_pkey PRIMARY KEY (id);


--
-- Name: vnd_limit_config vnd_limit_config_tenant_id_key; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.vnd_limit_config
    ADD CONSTRAINT vnd_limit_config_tenant_id_key UNIQUE (tenant_id);


--
-- Name: webauthn_challenges webauthn_challenges_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webauthn_challenges
    ADD CONSTRAINT webauthn_challenges_pkey PRIMARY KEY (id);


--
-- Name: webauthn_credentials webauthn_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webauthn_credentials
    ADD CONSTRAINT webauthn_credentials_pkey PRIMARY KEY (id);


--
-- Name: webhook_configs webhook_configs_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webhook_configs
    ADD CONSTRAINT webhook_configs_pkey PRIMARY KEY (id);


--
-- Name: webhook_events webhook_events_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webhook_events
    ADD CONSTRAINT webhook_events_pkey PRIMARY KEY (id);


--
-- Name: whitelisted_extension_actions whitelisted_extension_actions_pkey; Type: CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.whitelisted_extension_actions
    ADD CONSTRAINT whitelisted_extension_actions_pkey PRIMARY KEY (action_id);


--
-- Name: idx_aml_severity; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_aml_severity ON public.aml_cases USING btree (tenant_id, severity, status);


--
-- Name: idx_aml_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_aml_status ON public.aml_cases USING btree (tenant_id, status);


--
-- Name: idx_aml_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_aml_tenant ON public.aml_cases USING btree (tenant_id);


--
-- Name: idx_aml_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_aml_user ON public.aml_cases USING btree (tenant_id, user_id) WHERE (user_id IS NOT NULL);


--
-- Name: idx_audit_actor; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_audit_actor ON public.audit_log USING btree (tenant_id, actor_type, actor_id);


--
-- Name: idx_audit_resource; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_audit_resource ON public.audit_log USING btree (tenant_id, resource_type, resource_id);


--
-- Name: idx_audit_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_audit_tenant ON public.audit_log USING btree (tenant_id, created_at DESC);


--
-- Name: idx_balances_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_balances_user ON public.account_balances USING btree (tenant_id, user_id) WHERE (user_id IS NOT NULL);


--
-- Name: idx_bank_confirmations_bank_tx; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_bank_confirmations_bank_tx ON public.bank_confirmations USING btree (bank_tx_id) WHERE (bank_tx_id IS NOT NULL);


--
-- Name: idx_bank_confirmations_intent; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_bank_confirmations_intent ON public.bank_confirmations USING btree (matched_intent_id) WHERE (matched_intent_id IS NOT NULL);


--
-- Name: idx_bank_confirmations_provider; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_bank_confirmations_provider ON public.bank_confirmations USING btree (provider, created_at DESC);


--
-- Name: idx_bank_confirmations_reference; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_bank_confirmations_reference ON public.bank_confirmations USING btree (tenant_id, reference_code);


--
-- Name: idx_bank_confirmations_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_bank_confirmations_status ON public.bank_confirmations USING btree (status) WHERE ((status)::text = 'PENDING'::text);


--
-- Name: idx_bank_confirmations_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_bank_confirmations_tenant ON public.bank_confirmations USING btree (tenant_id);


--
-- Name: idx_bank_confirmations_unique; Type: INDEX; Schema: public; Owner: rampos
--

CREATE UNIQUE INDEX idx_bank_confirmations_unique ON public.bank_confirmations USING btree (provider, bank_tx_id) WHERE (bank_tx_id IS NOT NULL);


--
-- Name: idx_bank_webhook_secrets_lookup; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_bank_webhook_secrets_lookup ON public.bank_webhook_secrets USING btree (tenant_id, provider) WHERE (is_active = true);


--
-- Name: idx_case_notes_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_case_notes_tenant ON public.case_notes USING btree (tenant_id);


--
-- Name: idx_compliance_audit_actor; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_compliance_audit_actor ON public.compliance_audit_log USING btree (tenant_id, actor_id, created_at DESC);


--
-- Name: idx_compliance_audit_event_type; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_compliance_audit_event_type ON public.compliance_audit_log USING btree (tenant_id, event_type, created_at DESC);


--
-- Name: idx_compliance_audit_resource; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_compliance_audit_resource ON public.compliance_audit_log USING btree (tenant_id, resource_type, resource_id);


--
-- Name: idx_compliance_audit_sequence; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_compliance_audit_sequence ON public.compliance_audit_log USING btree (tenant_id, sequence_number);


--
-- Name: idx_compliance_audit_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_compliance_audit_tenant ON public.compliance_audit_log USING btree (tenant_id, created_at DESC);


--
-- Name: idx_compliance_tx_tenant_user_time; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_compliance_tx_tenant_user_time ON public.compliance_transactions USING btree (tenant_id, user_id, created_at DESC);


--
-- Name: idx_compliance_tx_type; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_compliance_tx_type ON public.compliance_transactions USING btree (transaction_type);


--
-- Name: idx_config_bundle_exports_one_active_approved_default; Type: INDEX; Schema: public; Owner: rampos
--

CREATE UNIQUE INDEX idx_config_bundle_exports_one_active_approved_default ON public.config_bundle_exports USING btree ((1)) WHERE ((tenant_id IS NULL) AND (is_active = true) AND (approval_status = 'approved'::text));


--
-- Name: idx_config_bundle_exports_one_active_approved_per_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE UNIQUE INDEX idx_config_bundle_exports_one_active_approved_per_tenant ON public.config_bundle_exports USING btree (tenant_id) WHERE ((tenant_id IS NOT NULL) AND (is_active = true) AND (approval_status = 'approved'::text));


--
-- Name: idx_config_bundle_exports_tenant_active; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_config_bundle_exports_tenant_active ON public.config_bundle_exports USING btree (tenant_id, is_active, updated_at DESC);


--
-- Name: idx_corridor_compliance_hooks_pack; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_corridor_compliance_hooks_pack ON public.corridor_compliance_hooks USING btree (corridor_pack_id, hook_kind);


--
-- Name: idx_corridor_cutoff_policies_pack; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_corridor_cutoff_policies_pack ON public.corridor_cutoff_policies USING btree (corridor_pack_id);


--
-- Name: idx_corridor_eligibility_rules_pack_partner; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_corridor_eligibility_rules_pack_partner ON public.corridor_eligibility_rules USING btree (corridor_pack_id, partner_id);


--
-- Name: idx_corridor_fee_profiles_pack; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_corridor_fee_profiles_pack ON public.corridor_fee_profiles USING btree (corridor_pack_id);


--
-- Name: idx_corridor_pack_endpoints_pack_role; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_corridor_pack_endpoints_pack_role ON public.corridor_pack_endpoints USING btree (corridor_pack_id, endpoint_role);


--
-- Name: idx_corridor_packs_tenant_code; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_corridor_packs_tenant_code ON public.corridor_packs USING btree (tenant_id, corridor_code, lifecycle_state);


--
-- Name: idx_corridor_rollout_scopes_pack_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_corridor_rollout_scopes_pack_tenant ON public.corridor_rollout_scopes USING btree (corridor_pack_id, tenant_id, environment);


--
-- Name: idx_credential_references_partner_environment; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_credential_references_partner_environment ON public.credential_references USING btree (partner_id, environment, credential_kind);


--
-- Name: idx_custom_domains_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_custom_domains_status ON public.custom_domains USING btree (status);


--
-- Name: idx_custom_domains_tenant_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_custom_domains_tenant_id ON public.custom_domains USING btree (tenant_id);


--
-- Name: idx_identity_providers_tenant_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_identity_providers_tenant_id ON public.identity_providers USING btree (tenant_id);


--
-- Name: idx_intents_created; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_intents_created ON public.intents USING btree (tenant_id, created_at DESC);


--
-- Name: idx_intents_expires; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_intents_expires ON public.intents USING btree (expires_at) WHERE ((expires_at IS NOT NULL) AND ((state)::text <> ALL ((ARRAY['COMPLETED'::character varying, 'EXPIRED'::character varying, 'CANCELLED'::character varying])::text[])));


--
-- Name: idx_intents_idempotency; Type: INDEX; Schema: public; Owner: rampos
--

CREATE UNIQUE INDEX idx_intents_idempotency ON public.intents USING btree (tenant_id, idempotency_key) WHERE (idempotency_key IS NOT NULL);


--
-- Name: idx_intents_reference; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_intents_reference ON public.intents USING btree (tenant_id, reference_code) WHERE (reference_code IS NOT NULL);


--
-- Name: idx_intents_state; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_intents_state ON public.intents USING btree (tenant_id, state);


--
-- Name: idx_intents_tenant_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_intents_tenant_user ON public.intents USING btree (tenant_id, user_id);


--
-- Name: idx_intents_type; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_intents_type ON public.intents USING btree (tenant_id, intent_type);


--
-- Name: idx_invoices_tenant_period; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_invoices_tenant_period ON public.invoices USING btree (tenant_id, period_start);


--
-- Name: idx_kyb_edges_tenant_target; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyb_edges_tenant_target ON public.kyb_ownership_edges USING btree (tenant_id, target_id, edge_type);


--
-- Name: idx_kyb_entities_tenant_type; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyb_entities_tenant_type ON public.kyb_entities USING btree (tenant_id, entity_type, created_at DESC);


--
-- Name: idx_kyb_evidence_packages_entity; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyb_evidence_packages_entity ON public.kyb_evidence_packages USING btree (institution_entity_id, corridor_code);


--
-- Name: idx_kyb_evidence_packages_tenant_review; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyb_evidence_packages_tenant_review ON public.kyb_evidence_packages USING btree (tenant_id, review_status, created_at DESC);


--
-- Name: idx_kyb_evidence_sources_package; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyb_evidence_sources_package ON public.kyb_evidence_sources USING btree (package_id, source_kind, collected_at DESC);


--
-- Name: idx_kyb_ubo_evidence_links_package; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyb_ubo_evidence_links_package ON public.kyb_ubo_evidence_links USING btree (package_id, review_state, owner_entity_id);


--
-- Name: idx_kyc_passport_consent_target; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyc_passport_consent_target ON public.kyc_passport_consent_grants USING btree (tenant_id, target_tenant_id, consent_status);


--
-- Name: idx_kyc_passport_vault_tenant_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyc_passport_vault_tenant_user ON public.kyc_passport_vault USING btree (tenant_id, user_id, created_at DESC);


--
-- Name: idx_kyc_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyc_status ON public.kyc_records USING btree (tenant_id, status);


--
-- Name: idx_kyc_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_kyc_user ON public.kyc_records USING btree (tenant_id, user_id);


--
-- Name: idx_ledger_account; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_ledger_account ON public.ledger_entries USING btree (tenant_id, account_type, currency);


--
-- Name: idx_ledger_intent; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_ledger_intent ON public.ledger_entries USING btree (intent_id);


--
-- Name: idx_ledger_sequence; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_ledger_sequence ON public.ledger_entries USING btree (sequence);


--
-- Name: idx_ledger_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_ledger_tenant ON public.ledger_entries USING btree (tenant_id);


--
-- Name: idx_ledger_transaction; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_ledger_transaction ON public.ledger_entries USING btree (transaction_id);


--
-- Name: idx_ledger_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_ledger_user ON public.ledger_entries USING btree (tenant_id, user_id) WHERE (user_id IS NOT NULL);


--
-- Name: idx_license_requirements_mandatory; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_license_requirements_mandatory ON public.license_requirements USING btree (license_type_id, is_mandatory);


--
-- Name: idx_license_requirements_type; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_license_requirements_type ON public.license_requirements USING btree (license_type_id);


--
-- Name: idx_license_submissions_requirement; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_license_submissions_requirement ON public.license_submissions USING btree (requirement_id);


--
-- Name: idx_license_submissions_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_license_submissions_status ON public.license_submissions USING btree (status);


--
-- Name: idx_license_submissions_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_license_submissions_tenant ON public.license_submissions USING btree (tenant_id);


--
-- Name: idx_license_types_code; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_license_types_code ON public.license_types USING btree (code);


--
-- Name: idx_license_types_jurisdiction; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_license_types_jurisdiction ON public.license_types USING btree (jurisdiction);


--
-- Name: idx_lp_keys_hash; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_lp_keys_hash ON public.registered_lp_keys USING btree (tenant_id, key_hash);


--
-- Name: idx_lp_keys_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_lp_keys_tenant ON public.registered_lp_keys USING btree (tenant_id, is_active);


--
-- Name: idx_lp_reliability_snapshots_lp_window; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_lp_reliability_snapshots_lp_window ON public.lp_reliability_snapshots USING btree (tenant_id, lp_id, direction, window_kind, window_ended_at DESC);


--
-- Name: idx_lp_reliability_snapshots_metadata; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_lp_reliability_snapshots_metadata ON public.lp_reliability_snapshots USING gin (metadata);


--
-- Name: idx_lp_reliability_snapshots_score; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_lp_reliability_snapshots_score ON public.lp_reliability_snapshots USING btree (tenant_id, direction, window_kind, reliability_score DESC) WHERE (reliability_score IS NOT NULL);


--
-- Name: idx_lp_reliability_snapshots_window; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_lp_reliability_snapshots_window ON public.lp_reliability_snapshots USING btree (tenant_id, window_kind, window_ended_at DESC);


--
-- Name: idx_magic_link_tokens_email_created; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_magic_link_tokens_email_created ON public.magic_link_tokens USING btree (email, created_at);


--
-- Name: idx_magic_link_tokens_expires; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_magic_link_tokens_expires ON public.magic_link_tokens USING btree (expires_at);


--
-- Name: idx_magic_link_tokens_hash; Type: INDEX; Schema: public; Owner: rampos
--

CREATE UNIQUE INDEX idx_magic_link_tokens_hash ON public.magic_link_tokens USING btree (token_hash);


--
-- Name: idx_offramp_intents_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_offramp_intents_status ON public.offramp_intents USING btree (state);


--
-- Name: idx_offramp_intents_tenant_created; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_offramp_intents_tenant_created ON public.offramp_intents USING btree (tenant_id, created_at DESC);


--
-- Name: idx_offramp_intents_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_offramp_intents_user ON public.offramp_intents USING btree (tenant_id, user_id);


--
-- Name: idx_partner_approval_references_tenant_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_partner_approval_references_tenant_status ON public.partner_approval_references USING btree (tenant_id, status, action_class);


--
-- Name: idx_partner_capabilities_partner_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_partner_capabilities_partner_id ON public.partner_capabilities USING btree (partner_id, capability_family);


--
-- Name: idx_partner_health_signals_capability_observed; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_partner_health_signals_capability_observed ON public.partner_health_signals USING btree (partner_capability_id, observed_at DESC);


--
-- Name: idx_partner_rollout_scopes_capability_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_partner_rollout_scopes_capability_tenant ON public.partner_rollout_scopes USING btree (partner_capability_id, tenant_id, environment);


--
-- Name: idx_partners_tenant_code; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_partners_tenant_code ON public.partners USING btree (tenant_id, code);


--
-- Name: idx_payment_method_capabilities_corridor; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_payment_method_capabilities_corridor ON public.payment_method_capabilities USING btree (corridor_pack_id, method_family, settlement_direction);


--
-- Name: idx_payment_method_capabilities_partner_capability; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_payment_method_capabilities_partner_capability ON public.payment_method_capabilities USING btree (partner_capability_id, method_family) WHERE (partner_capability_id IS NOT NULL);


--
-- Name: idx_portal_kyc_docs_case; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_portal_kyc_docs_case ON public.portal_kyc_documents USING btree (case_id);


--
-- Name: idx_portal_kyc_docs_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_portal_kyc_docs_user ON public.portal_kyc_documents USING btree (user_id);


--
-- Name: idx_portal_kyc_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_portal_kyc_status ON public.portal_kyc_cases USING btree (status);


--
-- Name: idx_portal_kyc_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_portal_kyc_tenant ON public.portal_kyc_cases USING btree (tenant_id);


--
-- Name: idx_portal_kyc_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_portal_kyc_user ON public.portal_kyc_cases USING btree (user_id);


--
-- Name: idx_portal_users_email; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_portal_users_email ON public.portal_users USING btree (email);


--
-- Name: idx_portal_users_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_portal_users_tenant ON public.portal_users USING btree (tenant_id);


--
-- Name: idx_pricing_plans_name; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_pricing_plans_name ON public.pricing_plans USING btree (name);


--
-- Name: idx_provider_routing_policies_lookup; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_provider_routing_policies_lookup ON public.provider_routing_policies USING btree (tenant_id, provider_family, corridor_code, entity_type, risk_tier, partner_key, asset_code);


--
-- Name: idx_provider_routing_policies_state; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_provider_routing_policies_state ON public.provider_routing_policies USING btree (tenant_id, provider_family, lifecycle_state, policy_name);


--
-- Name: idx_recon_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_recon_status ON public.recon_batches USING btree (status) WHERE ((status)::text = 'PENDING'::text);


--
-- Name: idx_recon_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_recon_tenant ON public.recon_batches USING btree (tenant_id, created_at DESC);


--
-- Name: idx_refresh_tokens_expires; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_refresh_tokens_expires ON public.refresh_tokens USING btree (expires_at);


--
-- Name: idx_refresh_tokens_family_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_refresh_tokens_family_id ON public.refresh_tokens USING btree (family_id);


--
-- Name: idx_refresh_tokens_hash; Type: INDEX; Schema: public; Owner: rampos
--

CREATE UNIQUE INDEX idx_refresh_tokens_hash ON public.refresh_tokens USING btree (token_hash);


--
-- Name: idx_refresh_tokens_user_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_refresh_tokens_user_id ON public.refresh_tokens USING btree (user_id);


--
-- Name: idx_rescreening_runs_alert_codes; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rescreening_runs_alert_codes ON public.compliance_rescreening_runs USING gin (alert_codes);


--
-- Name: idx_rescreening_runs_next_due; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rescreening_runs_next_due ON public.compliance_rescreening_runs USING btree (tenant_id, next_run_at) WHERE (next_run_at IS NOT NULL);


--
-- Name: idx_rescreening_runs_tenant_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rescreening_runs_tenant_status ON public.compliance_rescreening_runs USING btree (tenant_id, status, priority, scheduled_for DESC);


--
-- Name: idx_rescreening_runs_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rescreening_runs_user ON public.compliance_rescreening_runs USING btree (tenant_id, user_id, scheduled_for DESC);


--
-- Name: idx_rfq_bids_lp; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rfq_bids_lp ON public.rfq_bids USING btree (tenant_id, lp_id, created_at DESC);


--
-- Name: idx_rfq_bids_rfq_rate; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rfq_bids_rfq_rate ON public.rfq_bids USING btree (rfq_id, state, exchange_rate DESC);


--
-- Name: idx_rfq_requests_offramp; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rfq_requests_offramp ON public.rfq_requests USING btree (offramp_id) WHERE (offramp_id IS NOT NULL);


--
-- Name: idx_rfq_requests_tenant_state; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rfq_requests_tenant_state ON public.rfq_requests USING btree (tenant_id, state, created_at DESC);


--
-- Name: idx_rfq_requests_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rfq_requests_user ON public.rfq_requests USING btree (tenant_id, user_id);


--
-- Name: idx_risk_score_history_rule_version; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_risk_score_history_rule_version ON public.risk_score_history USING btree (rule_version_id, created_at DESC) WHERE (rule_version_id IS NOT NULL);


--
-- Name: idx_risk_score_history_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_risk_score_history_tenant ON public.risk_score_history USING btree (tenant_id);


--
-- Name: idx_rule_versions_active; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rule_versions_active ON public.aml_rule_versions USING btree (tenant_id) WHERE (is_active = true);


--
-- Name: idx_rule_versions_state; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rule_versions_state ON public.aml_rule_versions USING btree (tenant_id, version_state, version_number DESC);


--
-- Name: idx_rule_versions_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_rule_versions_tenant ON public.aml_rule_versions USING btree (tenant_id, version_number DESC);


--
-- Name: idx_sandbox_preset_scenarios_lookup; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_sandbox_preset_scenarios_lookup ON public.sandbox_preset_scenarios USING btree (preset_id, sort_order, scenario_code);


--
-- Name: idx_sandbox_presets_active; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_sandbox_presets_active ON public.sandbox_presets USING btree (is_active, preset_code);


--
-- Name: idx_sandbox_presets_metadata; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_sandbox_presets_metadata ON public.sandbox_presets USING gin (metadata);


--
-- Name: idx_score_history_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_score_history_user ON public.risk_score_history USING btree (user_id, created_at DESC);


--
-- Name: idx_settlements_offramp; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_settlements_offramp ON public.settlements USING btree (offramp_intent_id);


--
-- Name: idx_settlements_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_settlements_status ON public.settlements USING btree (status);


--
-- Name: idx_smart_accounts_address_chain; Type: INDEX; Schema: public; Owner: rampos
--

CREATE UNIQUE INDEX idx_smart_accounts_address_chain ON public.smart_accounts USING btree (address, chain_id);


--
-- Name: idx_smart_accounts_chain; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_smart_accounts_chain ON public.smart_accounts USING btree (chain_id);


--
-- Name: idx_smart_accounts_owner; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_smart_accounts_owner ON public.smart_accounts USING btree (owner_address);


--
-- Name: idx_smart_accounts_tenant_address; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_smart_accounts_tenant_address ON public.smart_accounts USING btree (tenant_id, address);


--
-- Name: idx_smart_accounts_tenant_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_smart_accounts_tenant_user ON public.smart_accounts USING btree (tenant_id, user_id);


--
-- Name: idx_sso_sessions_expires_at; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_sso_sessions_expires_at ON public.sso_sessions USING btree (expires_at);


--
-- Name: idx_sso_sessions_provider_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_sso_sessions_provider_id ON public.sso_sessions USING btree (provider_id);


--
-- Name: idx_sso_sessions_user_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_sso_sessions_user_id ON public.sso_sessions USING btree (user_id);


--
-- Name: idx_supported_tokens_symbol; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_supported_tokens_symbol ON public.supported_tokens USING btree (symbol);


--
-- Name: idx_supported_tokens_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_supported_tokens_tenant ON public.supported_tokens USING btree (tenant_id);


--
-- Name: idx_tenant_license_documents_license; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_license_documents_license ON public.tenant_license_documents USING btree (tenant_license_id);


--
-- Name: idx_tenant_license_documents_requirement; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_license_documents_requirement ON public.tenant_license_documents USING btree (requirement_id);


--
-- Name: idx_tenant_license_documents_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_license_documents_status ON public.tenant_license_documents USING btree (tenant_license_id, status);


--
-- Name: idx_tenant_license_documents_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_license_documents_tenant ON public.tenant_license_documents USING btree (tenant_id);


--
-- Name: idx_tenant_license_status_expiry; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_license_status_expiry ON public.tenant_license_status USING btree (expiry_date) WHERE (expiry_date IS NOT NULL);


--
-- Name: idx_tenant_license_status_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_license_status_status ON public.tenant_license_status USING btree (status);


--
-- Name: idx_tenant_license_status_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_license_status_tenant ON public.tenant_license_status USING btree (tenant_id);


--
-- Name: idx_tenant_licenses_expires; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_licenses_expires ON public.tenant_licenses USING btree (expires_at) WHERE (expires_at IS NOT NULL);


--
-- Name: idx_tenant_licenses_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_licenses_status ON public.tenant_licenses USING btree (tenant_id, status);


--
-- Name: idx_tenant_licenses_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_licenses_tenant ON public.tenant_licenses USING btree (tenant_id);


--
-- Name: idx_tenant_licenses_type; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_licenses_type ON public.tenant_licenses USING btree (license_type_id);


--
-- Name: idx_tenant_rate_limits_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenant_rate_limits_tenant ON public.tenant_rate_limits USING btree (tenant_id);


--
-- Name: idx_tenants_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tenants_status ON public.tenants USING btree (status);


--
-- Name: idx_token_balances_symbol; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_token_balances_symbol ON public.token_balances USING btree (symbol, chain_id);


--
-- Name: idx_token_balances_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_token_balances_user ON public.token_balances USING btree (tenant_id, user_id);


--
-- Name: idx_token_deployments_chain; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_token_deployments_chain ON public.token_chain_deployments USING btree (chain_id);


--
-- Name: idx_token_transactions_hash; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_token_transactions_hash ON public.token_transactions USING btree (tx_hash);


--
-- Name: idx_token_transactions_intent; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_token_transactions_intent ON public.token_transactions USING btree (intent_id);


--
-- Name: idx_token_transactions_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_token_transactions_status ON public.token_transactions USING btree (status, chain_id);


--
-- Name: idx_token_transactions_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_token_transactions_user ON public.token_transactions USING btree (tenant_id, user_id);


--
-- Name: idx_travel_rule_disclosures_correlation; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_disclosures_correlation ON public.travel_rule_disclosures USING btree (tenant_id, correlation_id) WHERE (correlation_id IS NOT NULL);


--
-- Name: idx_travel_rule_disclosures_intent; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_disclosures_intent ON public.travel_rule_disclosures USING btree (tenant_id, intent_id) WHERE (intent_id IS NOT NULL);


--
-- Name: idx_travel_rule_disclosures_payload; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_disclosures_payload ON public.travel_rule_disclosures USING gin (disclosure_payload);


--
-- Name: idx_travel_rule_disclosures_settlement; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_disclosures_settlement ON public.travel_rule_disclosures USING btree (tenant_id, settlement_id) WHERE (settlement_id IS NOT NULL);


--
-- Name: idx_travel_rule_disclosures_tenant_stage; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_disclosures_tenant_stage ON public.travel_rule_disclosures USING btree (tenant_id, lifecycle_stage, created_at DESC);


--
-- Name: idx_travel_rule_exception_queue_assignee; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_exception_queue_assignee ON public.travel_rule_exception_queue USING btree (tenant_id, assigned_to, queue_status) WHERE (assigned_to IS NOT NULL);


--
-- Name: idx_travel_rule_exception_queue_disclosure; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_exception_queue_disclosure ON public.travel_rule_exception_queue USING btree (tenant_id, disclosure_id);


--
-- Name: idx_travel_rule_exception_queue_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_exception_queue_status ON public.travel_rule_exception_queue USING btree (tenant_id, queue_status, severity, created_at DESC);


--
-- Name: idx_travel_rule_policies_asset_scope; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_policies_asset_scope ON public.travel_rule_policies USING gin (asset_scope);


--
-- Name: idx_travel_rule_policies_counterparty_scope; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_policies_counterparty_scope ON public.travel_rule_policies USING gin (counterparty_scope);


--
-- Name: idx_travel_rule_policies_jurisdiction; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_policies_jurisdiction ON public.travel_rule_policies USING btree (tenant_id, jurisdiction_code, direction_scope) WHERE (jurisdiction_code IS NOT NULL);


--
-- Name: idx_travel_rule_policies_tenant_active; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_policies_tenant_active ON public.travel_rule_policies USING btree (tenant_id, is_active, updated_at DESC);


--
-- Name: idx_travel_rule_transport_attempts_disclosure; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_transport_attempts_disclosure ON public.travel_rule_transport_attempts USING btree (tenant_id, disclosure_id, attempt_number DESC);


--
-- Name: idx_travel_rule_transport_attempts_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_transport_attempts_status ON public.travel_rule_transport_attempts USING btree (tenant_id, status, attempted_at DESC);


--
-- Name: idx_travel_rule_transport_attempts_transport_kind; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_transport_attempts_transport_kind ON public.travel_rule_transport_attempts USING btree (tenant_id, transport_kind, attempted_at DESC);


--
-- Name: idx_travel_rule_vasps_jurisdiction; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_vasps_jurisdiction ON public.travel_rule_vasps USING btree (tenant_id, jurisdiction_code) WHERE (jurisdiction_code IS NOT NULL);


--
-- Name: idx_travel_rule_vasps_metadata; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_vasps_metadata ON public.travel_rule_vasps USING gin (metadata);


--
-- Name: idx_travel_rule_vasps_tenant_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_travel_rule_vasps_tenant_status ON public.travel_rule_vasps USING btree (tenant_id, review_status, interoperability_status, updated_at DESC);


--
-- Name: idx_treasury_evidence_imports_account_scope; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_treasury_evidence_imports_account_scope ON public.treasury_evidence_imports USING btree (tenant_id, account_scope, asset_code, snapshot_at DESC);


--
-- Name: idx_treasury_evidence_imports_tenant_idempotency; Type: INDEX; Schema: public; Owner: rampos
--

CREATE UNIQUE INDEX idx_treasury_evidence_imports_tenant_idempotency ON public.treasury_evidence_imports USING btree (tenant_id, idempotency_key);


--
-- Name: idx_treasury_evidence_imports_tenant_source_snapshot; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_treasury_evidence_imports_tenant_source_snapshot ON public.treasury_evidence_imports USING btree (tenant_id, source_family, snapshot_at DESC);


--
-- Name: idx_tx_limit_history_daily; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tx_limit_history_daily ON public.transaction_limit_history USING btree (tenant_id, user_id, vietnam_date);


--
-- Name: idx_tx_limit_history_intent; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tx_limit_history_intent ON public.transaction_limit_history USING btree (intent_id);


--
-- Name: idx_tx_limit_history_monthly; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tx_limit_history_monthly ON public.transaction_limit_history USING btree (tenant_id, user_id, vietnam_month);


--
-- Name: idx_tx_limit_history_type; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_tx_limit_history_type ON public.transaction_limit_history USING btree (tenant_id, transaction_type);


--
-- Name: idx_usage_events_tenant_meter_time; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_usage_events_tenant_meter_time ON public.usage_events USING btree (tenant_id, meter_slug, "timestamp");


--
-- Name: idx_usage_events_tenant_time; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_usage_events_tenant_time ON public.usage_events USING btree (tenant_id, "timestamp");


--
-- Name: idx_user_limits_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_user_limits_tenant ON public.user_transaction_limits USING btree (tenant_id);


--
-- Name: idx_user_limits_tier; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_user_limits_tier ON public.user_transaction_limits USING btree (tenant_id, tier);


--
-- Name: idx_users_kyc_status; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_users_kyc_status ON public.users USING btree (tenant_id, kyc_status);


--
-- Name: idx_users_risk_score; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_users_risk_score ON public.users USING btree (tenant_id, risk_score DESC);


--
-- Name: idx_users_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_users_tenant ON public.users USING btree (tenant_id);


--
-- Name: idx_va_account; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_va_account ON public.virtual_accounts USING btree (bank_code, account_number);


--
-- Name: idx_va_tenant_user; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_va_tenant_user ON public.virtual_accounts USING btree (tenant_id, user_id);


--
-- Name: idx_vnd_limit_config_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_vnd_limit_config_tenant ON public.vnd_limit_config USING btree (tenant_id);


--
-- Name: idx_webauthn_challenges_expires; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webauthn_challenges_expires ON public.webauthn_challenges USING btree (expires_at);


--
-- Name: idx_webauthn_challenges_key; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webauthn_challenges_key ON public.webauthn_challenges USING btree (challenge_key);


--
-- Name: idx_webauthn_credentials_credential_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webauthn_credentials_credential_id ON public.webauthn_credentials USING btree (credential_id);


--
-- Name: idx_webauthn_credentials_tenant_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webauthn_credentials_tenant_id ON public.webauthn_credentials USING btree (tenant_id);


--
-- Name: idx_webauthn_credentials_user_id; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webauthn_credentials_user_id ON public.webauthn_credentials USING btree (user_id);


--
-- Name: idx_webhook_configs_active; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webhook_configs_active ON public.webhook_configs USING btree (tenant_id, active) WHERE (active = true);


--
-- Name: idx_webhook_configs_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webhook_configs_tenant ON public.webhook_configs USING btree (tenant_id);


--
-- Name: idx_webhooks_config; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webhooks_config ON public.webhook_events USING btree (config_id) WHERE (config_id IS NOT NULL);


--
-- Name: idx_webhooks_intent; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webhooks_intent ON public.webhook_events USING btree (intent_id);


--
-- Name: idx_webhooks_pending; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webhooks_pending ON public.webhook_events USING btree (next_attempt_at) WHERE ((status)::text = 'PENDING'::text);


--
-- Name: idx_webhooks_tenant; Type: INDEX; Schema: public; Owner: rampos
--

CREATE INDEX idx_webhooks_tenant ON public.webhook_events USING btree (tenant_id, created_at DESC);


--
-- Name: supported_tokens supported_tokens_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER supported_tokens_updated_at BEFORE UPDATE ON public.supported_tokens FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: token_balances token_balances_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER token_balances_updated_at BEFORE UPDATE ON public.token_balances FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: token_chain_deployments token_chain_deployments_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER token_chain_deployments_updated_at BEFORE UPDATE ON public.token_chain_deployments FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: token_transactions token_transactions_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER token_transactions_updated_at BEFORE UPDATE ON public.token_transactions FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: aml_cases trigger_aml_cases_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_aml_cases_updated_at BEFORE UPDATE ON public.aml_cases FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: bank_confirmations trigger_bank_confirmations_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_bank_confirmations_updated_at BEFORE UPDATE ON public.bank_confirmations FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: bank_webhook_secrets trigger_bank_webhook_secrets_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_bank_webhook_secrets_updated_at BEFORE UPDATE ON public.bank_webhook_secrets FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: compliance_rescreening_runs trigger_compliance_rescreening_runs_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_compliance_rescreening_runs_updated_at BEFORE UPDATE ON public.compliance_rescreening_runs FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: config_bundle_exports trigger_config_bundle_exports_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_config_bundle_exports_updated_at BEFORE UPDATE ON public.config_bundle_exports FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: corridor_compliance_hooks trigger_corridor_compliance_hooks_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_corridor_compliance_hooks_updated_at BEFORE UPDATE ON public.corridor_compliance_hooks FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: corridor_cutoff_policies trigger_corridor_cutoff_policies_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_corridor_cutoff_policies_updated_at BEFORE UPDATE ON public.corridor_cutoff_policies FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: corridor_eligibility_rules trigger_corridor_eligibility_rules_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_corridor_eligibility_rules_updated_at BEFORE UPDATE ON public.corridor_eligibility_rules FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: corridor_fee_profiles trigger_corridor_fee_profiles_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_corridor_fee_profiles_updated_at BEFORE UPDATE ON public.corridor_fee_profiles FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: corridor_pack_endpoints trigger_corridor_pack_endpoints_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_corridor_pack_endpoints_updated_at BEFORE UPDATE ON public.corridor_pack_endpoints FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: corridor_packs trigger_corridor_packs_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_corridor_packs_updated_at BEFORE UPDATE ON public.corridor_packs FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: corridor_rollout_scopes trigger_corridor_rollout_scopes_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_corridor_rollout_scopes_updated_at BEFORE UPDATE ON public.corridor_rollout_scopes FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: credential_references trigger_credential_references_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_credential_references_updated_at BEFORE UPDATE ON public.credential_references FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: intents trigger_intent_state_history; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_intent_state_history BEFORE UPDATE ON public.intents FOR EACH ROW EXECUTE FUNCTION public.append_state_history();


--
-- Name: intents trigger_intents_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_intents_updated_at BEFORE UPDATE ON public.intents FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: kyb_evidence_packages trigger_kyb_evidence_packages_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_kyb_evidence_packages_updated_at BEFORE UPDATE ON public.kyb_evidence_packages FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: kyb_evidence_sources trigger_kyb_evidence_sources_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_kyb_evidence_sources_updated_at BEFORE UPDATE ON public.kyb_evidence_sources FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: kyb_ubo_evidence_links trigger_kyb_ubo_evidence_links_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_kyb_ubo_evidence_links_updated_at BEFORE UPDATE ON public.kyb_ubo_evidence_links FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: license_requirements trigger_license_requirements_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_license_requirements_updated_at BEFORE UPDATE ON public.license_requirements FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: license_types trigger_license_types_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_license_types_updated_at BEFORE UPDATE ON public.license_types FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: registered_lp_keys trigger_lp_keys_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_lp_keys_updated_at BEFORE UPDATE ON public.registered_lp_keys FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: lp_reliability_snapshots trigger_lp_reliability_snapshots_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_lp_reliability_snapshots_updated_at BEFORE UPDATE ON public.lp_reliability_snapshots FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: partner_approval_references trigger_partner_approval_references_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_partner_approval_references_updated_at BEFORE UPDATE ON public.partner_approval_references FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: partner_capabilities trigger_partner_capabilities_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_partner_capabilities_updated_at BEFORE UPDATE ON public.partner_capabilities FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: partner_rollout_scopes trigger_partner_rollout_scopes_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_partner_rollout_scopes_updated_at BEFORE UPDATE ON public.partner_rollout_scopes FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: partners trigger_partners_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_partners_updated_at BEFORE UPDATE ON public.partners FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: payment_method_capabilities trigger_payment_method_capabilities_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_payment_method_capabilities_updated_at BEFORE UPDATE ON public.payment_method_capabilities FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: portal_kyc_cases trigger_portal_kyc_cases_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_portal_kyc_cases_updated_at BEFORE UPDATE ON public.portal_kyc_cases FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: compliance_audit_log trigger_prevent_audit_delete; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_prevent_audit_delete BEFORE DELETE ON public.compliance_audit_log FOR EACH ROW EXECUTE FUNCTION public.prevent_audit_modification();


--
-- Name: compliance_audit_log trigger_prevent_audit_update; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_prevent_audit_update BEFORE UPDATE ON public.compliance_audit_log FOR EACH ROW EXECUTE FUNCTION public.prevent_audit_modification();


--
-- Name: provider_routing_policies trigger_provider_routing_policies_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_provider_routing_policies_updated_at BEFORE UPDATE ON public.provider_routing_policies FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: rfq_requests trigger_rfq_requests_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_rfq_requests_updated_at BEFORE UPDATE ON public.rfq_requests FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: sandbox_presets trigger_sandbox_presets_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_sandbox_presets_updated_at BEFORE UPDATE ON public.sandbox_presets FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: smart_accounts trigger_smart_accounts_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_smart_accounts_updated_at BEFORE UPDATE ON public.smart_accounts FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: tenant_license_documents trigger_tenant_license_documents_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_tenant_license_documents_updated_at BEFORE UPDATE ON public.tenant_license_documents FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: tenant_licenses trigger_tenant_licenses_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_tenant_licenses_updated_at BEFORE UPDATE ON public.tenant_licenses FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: tenants trigger_tenants_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_tenants_updated_at BEFORE UPDATE ON public.tenants FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: travel_rule_disclosures trigger_travel_rule_disclosures_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_travel_rule_disclosures_updated_at BEFORE UPDATE ON public.travel_rule_disclosures FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: travel_rule_exception_queue trigger_travel_rule_exception_queue_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_travel_rule_exception_queue_updated_at BEFORE UPDATE ON public.travel_rule_exception_queue FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: travel_rule_policies trigger_travel_rule_policies_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_travel_rule_policies_updated_at BEFORE UPDATE ON public.travel_rule_policies FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: travel_rule_vasps trigger_travel_rule_vasps_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_travel_rule_vasps_updated_at BEFORE UPDATE ON public.travel_rule_vasps FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: treasury_evidence_imports trigger_treasury_evidence_imports_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_treasury_evidence_imports_updated_at BEFORE UPDATE ON public.treasury_evidence_imports FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: user_transaction_limits trigger_user_limits_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_user_limits_updated_at BEFORE UPDATE ON public.user_transaction_limits FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: users trigger_users_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_users_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: vnd_limit_config trigger_vnd_limit_config_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_vnd_limit_config_updated_at BEFORE UPDATE ON public.vnd_limit_config FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: webhook_configs trigger_webhook_configs_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_webhook_configs_updated_at BEFORE UPDATE ON public.webhook_configs FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: whitelisted_extension_actions trigger_whitelisted_extension_actions_updated_at; Type: TRIGGER; Schema: public; Owner: rampos
--

CREATE TRIGGER trigger_whitelisted_extension_actions_updated_at BEFORE UPDATE ON public.whitelisted_extension_actions FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();


--
-- Name: account_balances account_balances_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.account_balances
    ADD CONSTRAINT account_balances_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: aml_cases aml_cases_intent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.aml_cases
    ADD CONSTRAINT aml_cases_intent_id_fkey FOREIGN KEY (intent_id) REFERENCES public.intents(id);


--
-- Name: aml_cases aml_cases_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.aml_cases
    ADD CONSTRAINT aml_cases_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: aml_rule_versions aml_rule_versions_parent_version_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.aml_rule_versions
    ADD CONSTRAINT aml_rule_versions_parent_version_id_fkey FOREIGN KEY (parent_version_id) REFERENCES public.aml_rule_versions(id);


--
-- Name: aml_rule_versions aml_rule_versions_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.aml_rule_versions
    ADD CONSTRAINT aml_rule_versions_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: bank_confirmations bank_confirmations_matched_intent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.bank_confirmations
    ADD CONSTRAINT bank_confirmations_matched_intent_id_fkey FOREIGN KEY (matched_intent_id) REFERENCES public.intents(id);


--
-- Name: bank_confirmations bank_confirmations_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.bank_confirmations
    ADD CONSTRAINT bank_confirmations_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: bank_webhook_secrets bank_webhook_secrets_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.bank_webhook_secrets
    ADD CONSTRAINT bank_webhook_secrets_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: case_notes case_notes_case_fk; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.case_notes
    ADD CONSTRAINT case_notes_case_fk FOREIGN KEY (case_id) REFERENCES public.aml_cases(id);


--
-- Name: compliance_audit_log compliance_audit_log_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.compliance_audit_log
    ADD CONSTRAINT compliance_audit_log_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: compliance_rescreening_runs compliance_rescreening_runs_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.compliance_rescreening_runs
    ADD CONSTRAINT compliance_rescreening_runs_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: config_bundle_exports config_bundle_exports_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.config_bundle_exports
    ADD CONSTRAINT config_bundle_exports_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: corridor_compliance_hooks corridor_compliance_hooks_corridor_pack_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_compliance_hooks
    ADD CONSTRAINT corridor_compliance_hooks_corridor_pack_id_fkey FOREIGN KEY (corridor_pack_id) REFERENCES public.corridor_packs(id) ON DELETE CASCADE;


--
-- Name: corridor_cutoff_policies corridor_cutoff_policies_corridor_pack_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_cutoff_policies
    ADD CONSTRAINT corridor_cutoff_policies_corridor_pack_id_fkey FOREIGN KEY (corridor_pack_id) REFERENCES public.corridor_packs(id) ON DELETE CASCADE;


--
-- Name: corridor_eligibility_rules corridor_eligibility_rules_corridor_pack_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_eligibility_rules
    ADD CONSTRAINT corridor_eligibility_rules_corridor_pack_id_fkey FOREIGN KEY (corridor_pack_id) REFERENCES public.corridor_packs(id) ON DELETE CASCADE;


--
-- Name: corridor_eligibility_rules corridor_eligibility_rules_partner_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_eligibility_rules
    ADD CONSTRAINT corridor_eligibility_rules_partner_id_fkey FOREIGN KEY (partner_id) REFERENCES public.partners(id) ON DELETE SET NULL;


--
-- Name: corridor_fee_profiles corridor_fee_profiles_corridor_pack_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_fee_profiles
    ADD CONSTRAINT corridor_fee_profiles_corridor_pack_id_fkey FOREIGN KEY (corridor_pack_id) REFERENCES public.corridor_packs(id) ON DELETE CASCADE;


--
-- Name: corridor_pack_endpoints corridor_pack_endpoints_corridor_pack_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_pack_endpoints
    ADD CONSTRAINT corridor_pack_endpoints_corridor_pack_id_fkey FOREIGN KEY (corridor_pack_id) REFERENCES public.corridor_packs(id) ON DELETE CASCADE;


--
-- Name: corridor_pack_endpoints corridor_pack_endpoints_partner_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_pack_endpoints
    ADD CONSTRAINT corridor_pack_endpoints_partner_id_fkey FOREIGN KEY (partner_id) REFERENCES public.partners(id) ON DELETE SET NULL;


--
-- Name: corridor_packs corridor_packs_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_packs
    ADD CONSTRAINT corridor_packs_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: corridor_rollout_scopes corridor_rollout_scopes_corridor_pack_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_rollout_scopes
    ADD CONSTRAINT corridor_rollout_scopes_corridor_pack_id_fkey FOREIGN KEY (corridor_pack_id) REFERENCES public.corridor_packs(id) ON DELETE CASCADE;


--
-- Name: corridor_rollout_scopes corridor_rollout_scopes_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.corridor_rollout_scopes
    ADD CONSTRAINT corridor_rollout_scopes_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: credential_references credential_references_partner_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.credential_references
    ADD CONSTRAINT credential_references_partner_id_fkey FOREIGN KEY (partner_id) REFERENCES public.partners(id) ON DELETE CASCADE;


--
-- Name: custom_domains custom_domains_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.custom_domains
    ADD CONSTRAINT custom_domains_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: daily_usage daily_usage_meter_slug_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.daily_usage
    ADD CONSTRAINT daily_usage_meter_slug_fkey FOREIGN KEY (meter_slug) REFERENCES public.billing_meters(slug);


--
-- Name: daily_usage daily_usage_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.daily_usage
    ADD CONSTRAINT daily_usage_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: credential_references fk_credential_references_approval_reference; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.credential_references
    ADD CONSTRAINT fk_credential_references_approval_reference FOREIGN KEY (approval_reference) REFERENCES public.partner_approval_references(id);


--
-- Name: partner_rollout_scopes fk_partner_rollout_scopes_approval_reference; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_rollout_scopes
    ADD CONSTRAINT fk_partner_rollout_scopes_approval_reference FOREIGN KEY (approval_reference) REFERENCES public.partner_approval_references(id);


--
-- Name: usage_events fk_usage_tenant; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.usage_events
    ADD CONSTRAINT fk_usage_tenant FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: token_balances fk_user; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_balances
    ADD CONSTRAINT fk_user FOREIGN KEY (tenant_id, user_id) REFERENCES public.users(tenant_id, id) ON DELETE CASCADE;


--
-- Name: token_transactions fk_user_tx; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_transactions
    ADD CONSTRAINT fk_user_tx FOREIGN KEY (tenant_id, user_id) REFERENCES public.users(tenant_id, id) ON DELETE CASCADE;


--
-- Name: identity_providers identity_providers_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.identity_providers
    ADD CONSTRAINT identity_providers_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: intents intents_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.intents
    ADD CONSTRAINT intents_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: invoices invoices_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: kyb_evidence_packages kyb_evidence_packages_institution_entity_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_evidence_packages
    ADD CONSTRAINT kyb_evidence_packages_institution_entity_id_fkey FOREIGN KEY (institution_entity_id) REFERENCES public.kyb_entities(id) ON DELETE CASCADE;


--
-- Name: kyb_evidence_packages kyb_evidence_packages_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_evidence_packages
    ADD CONSTRAINT kyb_evidence_packages_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: kyb_evidence_sources kyb_evidence_sources_package_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_evidence_sources
    ADD CONSTRAINT kyb_evidence_sources_package_id_fkey FOREIGN KEY (package_id) REFERENCES public.kyb_evidence_packages(id) ON DELETE CASCADE;


--
-- Name: kyb_ubo_evidence_links kyb_ubo_evidence_links_owner_entity_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_ubo_evidence_links
    ADD CONSTRAINT kyb_ubo_evidence_links_owner_entity_id_fkey FOREIGN KEY (owner_entity_id) REFERENCES public.kyb_entities(id) ON DELETE CASCADE;


--
-- Name: kyb_ubo_evidence_links kyb_ubo_evidence_links_package_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyb_ubo_evidence_links
    ADD CONSTRAINT kyb_ubo_evidence_links_package_id_fkey FOREIGN KEY (package_id) REFERENCES public.kyb_evidence_packages(id) ON DELETE CASCADE;


--
-- Name: kyc_passport_consent_grants kyc_passport_consent_grants_passport_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyc_passport_consent_grants
    ADD CONSTRAINT kyc_passport_consent_grants_passport_id_fkey FOREIGN KEY (passport_id) REFERENCES public.kyc_passport_vault(id) ON DELETE CASCADE;


--
-- Name: kyc_records kyc_records_tenant_id_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.kyc_records
    ADD CONSTRAINT kyc_records_tenant_id_user_id_fkey FOREIGN KEY (tenant_id, user_id) REFERENCES public.users(tenant_id, id);


--
-- Name: ledger_entries ledger_entries_intent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.ledger_entries
    ADD CONSTRAINT ledger_entries_intent_id_fkey FOREIGN KEY (intent_id) REFERENCES public.intents(id);


--
-- Name: ledger_entries ledger_entries_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.ledger_entries
    ADD CONSTRAINT ledger_entries_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: license_requirements license_requirements_license_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.license_requirements
    ADD CONSTRAINT license_requirements_license_type_id_fkey FOREIGN KEY (license_type_id) REFERENCES public.license_types(id) ON DELETE CASCADE;


--
-- Name: license_submissions license_submissions_requirement_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.license_submissions
    ADD CONSTRAINT license_submissions_requirement_id_fkey FOREIGN KEY (requirement_id) REFERENCES public.license_requirements(id);


--
-- Name: license_submissions license_submissions_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.license_submissions
    ADD CONSTRAINT license_submissions_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: lp_reliability_snapshots lp_reliability_snapshots_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.lp_reliability_snapshots
    ADD CONSTRAINT lp_reliability_snapshots_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: offramp_intents offramp_intents_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.offramp_intents
    ADD CONSTRAINT offramp_intents_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: partner_approval_references partner_approval_references_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_approval_references
    ADD CONSTRAINT partner_approval_references_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: partner_capabilities partner_capabilities_partner_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_capabilities
    ADD CONSTRAINT partner_capabilities_partner_id_fkey FOREIGN KEY (partner_id) REFERENCES public.partners(id) ON DELETE CASCADE;


--
-- Name: partner_health_signals partner_health_signals_partner_capability_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_health_signals
    ADD CONSTRAINT partner_health_signals_partner_capability_id_fkey FOREIGN KEY (partner_capability_id) REFERENCES public.partner_capabilities(id) ON DELETE CASCADE;


--
-- Name: partner_rollout_scopes partner_rollout_scopes_partner_capability_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_rollout_scopes
    ADD CONSTRAINT partner_rollout_scopes_partner_capability_id_fkey FOREIGN KEY (partner_capability_id) REFERENCES public.partner_capabilities(id) ON DELETE CASCADE;


--
-- Name: partner_rollout_scopes partner_rollout_scopes_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partner_rollout_scopes
    ADD CONSTRAINT partner_rollout_scopes_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: partners partners_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.partners
    ADD CONSTRAINT partners_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: payment_method_capabilities payment_method_capabilities_corridor_pack_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.payment_method_capabilities
    ADD CONSTRAINT payment_method_capabilities_corridor_pack_id_fkey FOREIGN KEY (corridor_pack_id) REFERENCES public.corridor_packs(id) ON DELETE CASCADE;


--
-- Name: payment_method_capabilities payment_method_capabilities_partner_capability_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.payment_method_capabilities
    ADD CONSTRAINT payment_method_capabilities_partner_capability_id_fkey FOREIGN KEY (partner_capability_id) REFERENCES public.partner_capabilities(id) ON DELETE SET NULL;


--
-- Name: portal_kyc_cases portal_kyc_cases_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.portal_kyc_cases
    ADD CONSTRAINT portal_kyc_cases_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.portal_users(id);


--
-- Name: portal_kyc_documents portal_kyc_documents_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.portal_kyc_documents
    ADD CONSTRAINT portal_kyc_documents_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.portal_kyc_cases(id) ON DELETE CASCADE;


--
-- Name: portal_kyc_documents portal_kyc_documents_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.portal_kyc_documents
    ADD CONSTRAINT portal_kyc_documents_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.portal_users(id);


--
-- Name: provider_routing_policies provider_routing_policies_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.provider_routing_policies
    ADD CONSTRAINT provider_routing_policies_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: rails_adapters rails_adapters_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rails_adapters
    ADD CONSTRAINT rails_adapters_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: recon_batches recon_batches_rails_adapter_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.recon_batches
    ADD CONSTRAINT recon_batches_rails_adapter_id_fkey FOREIGN KEY (rails_adapter_id) REFERENCES public.rails_adapters(id);


--
-- Name: recon_batches recon_batches_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.recon_batches
    ADD CONSTRAINT recon_batches_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: registered_lp_keys registered_lp_keys_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.registered_lp_keys
    ADD CONSTRAINT registered_lp_keys_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: rfq_bids rfq_bids_rfq_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rfq_bids
    ADD CONSTRAINT rfq_bids_rfq_id_fkey FOREIGN KEY (rfq_id) REFERENCES public.rfq_requests(id);


--
-- Name: rfq_bids rfq_bids_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rfq_bids
    ADD CONSTRAINT rfq_bids_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: rfq_requests rfq_requests_offramp_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rfq_requests
    ADD CONSTRAINT rfq_requests_offramp_id_fkey FOREIGN KEY (offramp_id) REFERENCES public.offramp_intents(id);


--
-- Name: rfq_requests rfq_requests_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.rfq_requests
    ADD CONSTRAINT rfq_requests_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: risk_score_history risk_score_history_rule_version_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.risk_score_history
    ADD CONSTRAINT risk_score_history_rule_version_id_fkey FOREIGN KEY (rule_version_id) REFERENCES public.aml_rule_versions(id);


--
-- Name: risk_score_history risk_score_history_user_fk; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.risk_score_history
    ADD CONSTRAINT risk_score_history_user_fk FOREIGN KEY (tenant_id, user_id) REFERENCES public.users(tenant_id, id);


--
-- Name: sandbox_preset_scenarios sandbox_preset_scenarios_preset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.sandbox_preset_scenarios
    ADD CONSTRAINT sandbox_preset_scenarios_preset_id_fkey FOREIGN KEY (preset_id) REFERENCES public.sandbox_presets(id) ON DELETE CASCADE;


--
-- Name: smart_accounts smart_accounts_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.smart_accounts
    ADD CONSTRAINT smart_accounts_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: sso_sessions sso_sessions_provider_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.sso_sessions
    ADD CONSTRAINT sso_sessions_provider_id_fkey FOREIGN KEY (provider_id) REFERENCES public.identity_providers(id);


--
-- Name: supported_tokens supported_tokens_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.supported_tokens
    ADD CONSTRAINT supported_tokens_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: tenant_license_documents tenant_license_documents_requirement_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_license_documents
    ADD CONSTRAINT tenant_license_documents_requirement_id_fkey FOREIGN KEY (requirement_id) REFERENCES public.license_requirements(id);


--
-- Name: tenant_license_documents tenant_license_documents_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_license_documents
    ADD CONSTRAINT tenant_license_documents_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: tenant_license_documents tenant_license_documents_tenant_license_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_license_documents
    ADD CONSTRAINT tenant_license_documents_tenant_license_id_fkey FOREIGN KEY (tenant_license_id) REFERENCES public.tenant_licenses(id) ON DELETE CASCADE;


--
-- Name: tenant_license_status tenant_license_status_requirement_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_license_status
    ADD CONSTRAINT tenant_license_status_requirement_id_fkey FOREIGN KEY (requirement_id) REFERENCES public.license_requirements(id);


--
-- Name: tenant_license_status tenant_license_status_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_license_status
    ADD CONSTRAINT tenant_license_status_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: tenant_licenses tenant_licenses_license_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_licenses
    ADD CONSTRAINT tenant_licenses_license_type_id_fkey FOREIGN KEY (license_type_id) REFERENCES public.license_types(id);


--
-- Name: tenant_licenses tenant_licenses_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_licenses
    ADD CONSTRAINT tenant_licenses_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: tenant_rate_limits tenant_rate_limits_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.tenant_rate_limits
    ADD CONSTRAINT tenant_rate_limits_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: token_balances token_balances_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_balances
    ADD CONSTRAINT token_balances_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: token_chain_deployments token_chain_deployments_token_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_chain_deployments
    ADD CONSTRAINT token_chain_deployments_token_id_fkey FOREIGN KEY (token_id) REFERENCES public.supported_tokens(id) ON DELETE CASCADE;


--
-- Name: token_transactions token_transactions_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.token_transactions
    ADD CONSTRAINT token_transactions_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: transaction_limit_history transaction_limit_history_intent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.transaction_limit_history
    ADD CONSTRAINT transaction_limit_history_intent_id_fkey FOREIGN KEY (intent_id) REFERENCES public.intents(id);


--
-- Name: transaction_limit_history transaction_limit_history_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.transaction_limit_history
    ADD CONSTRAINT transaction_limit_history_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: transaction_limit_history transaction_limit_history_tenant_id_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.transaction_limit_history
    ADD CONSTRAINT transaction_limit_history_tenant_id_user_id_fkey FOREIGN KEY (tenant_id, user_id) REFERENCES public.users(tenant_id, id);


--
-- Name: travel_rule_disclosures travel_rule_disclosures_beneficiary_vasp_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_disclosures
    ADD CONSTRAINT travel_rule_disclosures_beneficiary_vasp_id_fkey FOREIGN KEY (beneficiary_vasp_id) REFERENCES public.travel_rule_vasps(id);


--
-- Name: travel_rule_disclosures travel_rule_disclosures_originator_vasp_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_disclosures
    ADD CONSTRAINT travel_rule_disclosures_originator_vasp_id_fkey FOREIGN KEY (originator_vasp_id) REFERENCES public.travel_rule_vasps(id);


--
-- Name: travel_rule_disclosures travel_rule_disclosures_policy_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_disclosures
    ADD CONSTRAINT travel_rule_disclosures_policy_id_fkey FOREIGN KEY (policy_id) REFERENCES public.travel_rule_policies(id);


--
-- Name: travel_rule_disclosures travel_rule_disclosures_settlement_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_disclosures
    ADD CONSTRAINT travel_rule_disclosures_settlement_id_fkey FOREIGN KEY (settlement_id) REFERENCES public.settlements(id);


--
-- Name: travel_rule_disclosures travel_rule_disclosures_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_disclosures
    ADD CONSTRAINT travel_rule_disclosures_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: travel_rule_exception_queue travel_rule_exception_queue_disclosure_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_exception_queue
    ADD CONSTRAINT travel_rule_exception_queue_disclosure_id_fkey FOREIGN KEY (disclosure_id) REFERENCES public.travel_rule_disclosures(id);


--
-- Name: travel_rule_exception_queue travel_rule_exception_queue_latest_attempt_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_exception_queue
    ADD CONSTRAINT travel_rule_exception_queue_latest_attempt_id_fkey FOREIGN KEY (latest_attempt_id) REFERENCES public.travel_rule_transport_attempts(id);


--
-- Name: travel_rule_exception_queue travel_rule_exception_queue_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_exception_queue
    ADD CONSTRAINT travel_rule_exception_queue_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: travel_rule_policies travel_rule_policies_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_policies
    ADD CONSTRAINT travel_rule_policies_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: travel_rule_transport_attempts travel_rule_transport_attempts_disclosure_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_transport_attempts
    ADD CONSTRAINT travel_rule_transport_attempts_disclosure_id_fkey FOREIGN KEY (disclosure_id) REFERENCES public.travel_rule_disclosures(id);


--
-- Name: travel_rule_transport_attempts travel_rule_transport_attempts_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_transport_attempts
    ADD CONSTRAINT travel_rule_transport_attempts_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: travel_rule_vasps travel_rule_vasps_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.travel_rule_vasps
    ADD CONSTRAINT travel_rule_vasps_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: treasury_evidence_imports treasury_evidence_imports_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.treasury_evidence_imports
    ADD CONSTRAINT treasury_evidence_imports_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


--
-- Name: usage_events usage_events_meter_slug_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.usage_events
    ADD CONSTRAINT usage_events_meter_slug_fkey FOREIGN KEY (meter_slug) REFERENCES public.billing_meters(slug);


--
-- Name: user_transaction_limits user_transaction_limits_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.user_transaction_limits
    ADD CONSTRAINT user_transaction_limits_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: user_transaction_limits user_transaction_limits_tenant_id_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.user_transaction_limits
    ADD CONSTRAINT user_transaction_limits_tenant_id_user_id_fkey FOREIGN KEY (tenant_id, user_id) REFERENCES public.users(tenant_id, id);


--
-- Name: users users_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: virtual_accounts virtual_accounts_rails_adapter_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.virtual_accounts
    ADD CONSTRAINT virtual_accounts_rails_adapter_id_fkey FOREIGN KEY (rails_adapter_id) REFERENCES public.rails_adapters(id);


--
-- Name: virtual_accounts virtual_accounts_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.virtual_accounts
    ADD CONSTRAINT virtual_accounts_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: vnd_limit_config vnd_limit_config_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.vnd_limit_config
    ADD CONSTRAINT vnd_limit_config_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: webhook_configs webhook_configs_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webhook_configs
    ADD CONSTRAINT webhook_configs_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: webhook_events webhook_events_config_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webhook_events
    ADD CONSTRAINT webhook_events_config_id_fkey FOREIGN KEY (config_id) REFERENCES public.webhook_configs(id);


--
-- Name: webhook_events webhook_events_intent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webhook_events
    ADD CONSTRAINT webhook_events_intent_id_fkey FOREIGN KEY (intent_id) REFERENCES public.intents(id);


--
-- Name: webhook_events webhook_events_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: rampos
--

ALTER TABLE ONLY public.webhook_events
    ADD CONSTRAINT webhook_events_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id);


--
-- Name: account_balances; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.account_balances ENABLE ROW LEVEL SECURITY;

--
-- Name: aml_cases; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.aml_cases ENABLE ROW LEVEL SECURITY;

--
-- Name: aml_rule_versions; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.aml_rule_versions ENABLE ROW LEVEL SECURITY;

--
-- Name: audit_log; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.audit_log ENABLE ROW LEVEL SECURITY;

--
-- Name: bank_confirmations; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.bank_confirmations ENABLE ROW LEVEL SECURITY;

--
-- Name: bank_confirmations bank_confirmations_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY bank_confirmations_tenant_isolation ON public.bank_confirmations USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: bank_webhook_secrets; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.bank_webhook_secrets ENABLE ROW LEVEL SECURITY;

--
-- Name: bank_webhook_secrets bank_webhook_secrets_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY bank_webhook_secrets_tenant_isolation ON public.bank_webhook_secrets USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: case_notes; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.case_notes ENABLE ROW LEVEL SECURITY;

--
-- Name: compliance_audit_log compliance_audit_insert_only; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY compliance_audit_insert_only ON public.compliance_audit_log FOR INSERT WITH CHECK (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: compliance_audit_log; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.compliance_audit_log ENABLE ROW LEVEL SECURITY;

--
-- Name: compliance_audit_log compliance_audit_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY compliance_audit_tenant_isolation ON public.compliance_audit_log USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: compliance_rescreening_runs; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.compliance_rescreening_runs ENABLE ROW LEVEL SECURITY;

--
-- Name: compliance_rescreening_runs compliance_rescreening_runs_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY compliance_rescreening_runs_tenant_isolation ON public.compliance_rescreening_runs USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: compliance_transactions; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.compliance_transactions ENABLE ROW LEVEL SECURITY;

--
-- Name: custom_domains; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.custom_domains ENABLE ROW LEVEL SECURITY;

--
-- Name: daily_usage; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.daily_usage ENABLE ROW LEVEL SECURITY;

--
-- Name: identity_providers; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.identity_providers ENABLE ROW LEVEL SECURITY;

--
-- Name: intents; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.intents ENABLE ROW LEVEL SECURITY;

--
-- Name: invoices; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.invoices ENABLE ROW LEVEL SECURITY;

--
-- Name: kyc_records; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.kyc_records ENABLE ROW LEVEL SECURITY;

--
-- Name: ledger_entries; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.ledger_entries ENABLE ROW LEVEL SECURITY;

--
-- Name: license_submissions; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.license_submissions ENABLE ROW LEVEL SECURITY;

--
-- Name: license_submissions license_submissions_insert; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY license_submissions_insert ON public.license_submissions FOR INSERT WITH CHECK ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: license_submissions license_submissions_select; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY license_submissions_select ON public.license_submissions FOR SELECT USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: license_submissions license_submissions_update; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY license_submissions_update ON public.license_submissions FOR UPDATE USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: registered_lp_keys lp_keys_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY lp_keys_tenant_isolation ON public.registered_lp_keys USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: lp_reliability_snapshots; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.lp_reliability_snapshots ENABLE ROW LEVEL SECURITY;

--
-- Name: lp_reliability_snapshots lp_reliability_snapshots_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY lp_reliability_snapshots_tenant_isolation ON public.lp_reliability_snapshots USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: offramp_intents; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.offramp_intents ENABLE ROW LEVEL SECURITY;

--
-- Name: offramp_intents offramp_intents_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY offramp_intents_tenant_isolation ON public.offramp_intents USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: rails_adapters; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.rails_adapters ENABLE ROW LEVEL SECURITY;

--
-- Name: recon_batches; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.recon_batches ENABLE ROW LEVEL SECURITY;

--
-- Name: registered_lp_keys; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.registered_lp_keys ENABLE ROW LEVEL SECURITY;

--
-- Name: rfq_bids; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.rfq_bids ENABLE ROW LEVEL SECURITY;

--
-- Name: rfq_bids rfq_bids_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY rfq_bids_tenant_isolation ON public.rfq_bids USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: rfq_requests; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.rfq_requests ENABLE ROW LEVEL SECURITY;

--
-- Name: rfq_requests rfq_requests_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY rfq_requests_tenant_isolation ON public.rfq_requests USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: risk_score_history; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.risk_score_history ENABLE ROW LEVEL SECURITY;

--
-- Name: smart_accounts; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.smart_accounts ENABLE ROW LEVEL SECURITY;

--
-- Name: smart_accounts smart_accounts_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY smart_accounts_tenant_isolation ON public.smart_accounts USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: sso_sessions; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.sso_sessions ENABLE ROW LEVEL SECURITY;

--
-- Name: supported_tokens; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.supported_tokens ENABLE ROW LEVEL SECURITY;

--
-- Name: supported_tokens supported_tokens_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY supported_tokens_tenant_isolation ON public.supported_tokens USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: account_balances tenant_isolation_account_balances; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_account_balances ON public.account_balances USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: aml_cases tenant_isolation_aml_cases; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_aml_cases ON public.aml_cases USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: aml_rule_versions tenant_isolation_aml_rule_versions; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_aml_rule_versions ON public.aml_rule_versions USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: audit_log tenant_isolation_audit_log; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_audit_log ON public.audit_log USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: case_notes tenant_isolation_case_notes; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_case_notes ON public.case_notes USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: compliance_transactions tenant_isolation_compliance_transactions; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_compliance_transactions ON public.compliance_transactions USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: custom_domains tenant_isolation_custom_domains; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_custom_domains ON public.custom_domains USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: daily_usage tenant_isolation_daily_usage; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_daily_usage ON public.daily_usage USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: identity_providers tenant_isolation_idp; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_idp ON public.identity_providers USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: intents tenant_isolation_intents; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_intents ON public.intents USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: invoices tenant_isolation_invoices; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_invoices ON public.invoices USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: kyc_records tenant_isolation_kyc_records; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_kyc_records ON public.kyc_records USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: ledger_entries tenant_isolation_ledger_entries; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_ledger_entries ON public.ledger_entries USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: rails_adapters tenant_isolation_rails_adapters; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_rails_adapters ON public.rails_adapters USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: recon_batches tenant_isolation_recon_batches; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_recon_batches ON public.recon_batches USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: risk_score_history tenant_isolation_risk_score_history; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_risk_score_history ON public.risk_score_history USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: sso_sessions tenant_isolation_sso_sessions; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_sso_sessions ON public.sso_sessions USING (((provider_id)::text IN ( SELECT identity_providers.id
   FROM public.identity_providers
  WHERE ((identity_providers.tenant_id)::text = current_setting('app.current_tenant'::text, true)))));


--
-- Name: usage_events tenant_isolation_usage_events; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_usage_events ON public.usage_events USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: users tenant_isolation_users; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_users ON public.users USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: virtual_accounts tenant_isolation_virtual_accounts; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_virtual_accounts ON public.virtual_accounts USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: webhook_events tenant_isolation_webhook_events; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_isolation_webhook_events ON public.webhook_events USING (((current_setting('app.current_tenant'::text, true) IS NOT NULL) AND ((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)));


--
-- Name: tenant_license_documents; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.tenant_license_documents ENABLE ROW LEVEL SECURITY;

--
-- Name: tenant_license_documents tenant_license_documents_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_license_documents_isolation ON public.tenant_license_documents USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: tenant_license_status; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.tenant_license_status ENABLE ROW LEVEL SECURITY;

--
-- Name: tenant_license_status tenant_license_status_insert; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_license_status_insert ON public.tenant_license_status FOR INSERT WITH CHECK ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: tenant_license_status tenant_license_status_select; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_license_status_select ON public.tenant_license_status FOR SELECT USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: tenant_license_status tenant_license_status_update; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_license_status_update ON public.tenant_license_status FOR UPDATE USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: tenant_licenses; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.tenant_licenses ENABLE ROW LEVEL SECURITY;

--
-- Name: tenant_licenses tenant_licenses_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tenant_licenses_isolation ON public.tenant_licenses USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: token_balances; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.token_balances ENABLE ROW LEVEL SECURITY;

--
-- Name: token_balances token_balances_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY token_balances_tenant_isolation ON public.token_balances USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: token_chain_deployments; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.token_chain_deployments ENABLE ROW LEVEL SECURITY;

--
-- Name: token_chain_deployments token_deployments_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY token_deployments_tenant_isolation ON public.token_chain_deployments USING ((token_id IN ( SELECT supported_tokens.id
   FROM public.supported_tokens
  WHERE ((supported_tokens.tenant_id)::text = current_setting('app.current_tenant'::text, true)))));


--
-- Name: token_transactions; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.token_transactions ENABLE ROW LEVEL SECURITY;

--
-- Name: token_transactions token_transactions_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY token_transactions_tenant_isolation ON public.token_transactions USING (((tenant_id)::text = current_setting('app.current_tenant'::text, true)));


--
-- Name: transaction_limit_history; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.transaction_limit_history ENABLE ROW LEVEL SECURITY;

--
-- Name: travel_rule_disclosures; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.travel_rule_disclosures ENABLE ROW LEVEL SECURITY;

--
-- Name: travel_rule_disclosures travel_rule_disclosures_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY travel_rule_disclosures_tenant_isolation ON public.travel_rule_disclosures USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: travel_rule_exception_queue; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.travel_rule_exception_queue ENABLE ROW LEVEL SECURITY;

--
-- Name: travel_rule_exception_queue travel_rule_exception_queue_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY travel_rule_exception_queue_tenant_isolation ON public.travel_rule_exception_queue USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: travel_rule_policies; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.travel_rule_policies ENABLE ROW LEVEL SECURITY;

--
-- Name: travel_rule_policies travel_rule_policies_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY travel_rule_policies_tenant_isolation ON public.travel_rule_policies USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: travel_rule_transport_attempts; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.travel_rule_transport_attempts ENABLE ROW LEVEL SECURITY;

--
-- Name: travel_rule_transport_attempts travel_rule_transport_attempts_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY travel_rule_transport_attempts_tenant_isolation ON public.travel_rule_transport_attempts USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: travel_rule_vasps; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.travel_rule_vasps ENABLE ROW LEVEL SECURITY;

--
-- Name: travel_rule_vasps travel_rule_vasps_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY travel_rule_vasps_tenant_isolation ON public.travel_rule_vasps USING ((tenant_id = current_setting('app.current_tenant'::text, true)));


--
-- Name: transaction_limit_history tx_history_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY tx_history_tenant_isolation ON public.transaction_limit_history USING (((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)) WITH CHECK (((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text));


--
-- Name: usage_events; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.usage_events ENABLE ROW LEVEL SECURITY;

--
-- Name: user_transaction_limits user_limits_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY user_limits_tenant_isolation ON public.user_transaction_limits USING (((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)) WITH CHECK (((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text));


--
-- Name: user_transaction_limits; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.user_transaction_limits ENABLE ROW LEVEL SECURITY;

--
-- Name: users; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;

--
-- Name: virtual_accounts; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.virtual_accounts ENABLE ROW LEVEL SECURITY;

--
-- Name: vnd_limit_config vnd_config_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY vnd_config_tenant_isolation ON public.vnd_limit_config USING (((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)) WITH CHECK (((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text));


--
-- Name: vnd_limit_config; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.vnd_limit_config ENABLE ROW LEVEL SECURITY;

--
-- Name: webhook_configs; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.webhook_configs ENABLE ROW LEVEL SECURITY;

--
-- Name: webhook_configs webhook_configs_tenant_isolation; Type: POLICY; Schema: public; Owner: rampos
--

CREATE POLICY webhook_configs_tenant_isolation ON public.webhook_configs USING (((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text)) WITH CHECK (((tenant_id)::text = ((current_setting('app.current_tenant'::text, true))::character varying)::text));


--
-- Name: webhook_events; Type: ROW SECURITY; Schema: public; Owner: rampos
--

ALTER TABLE public.webhook_events ENABLE ROW LEVEL SECURITY;

--
-- PostgreSQL database dump complete
--

\unrestrict AbNcwVUvibGCquh2BjBwf6v6t1zdOaRhhpddb3QjxcNh05SRx1mesGFfR7h5vEi

