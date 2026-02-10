import React, { useState, useEffect, useCallback } from 'react';
import type { KYCConfig, KYCResult, KYCCallbacks, KYCLevel, KYCStatus, KYCDocument, WidgetTheme } from '../types/index';
import { RampOSEventEmitter } from '../utils/events';
import { resolveTheme } from './shared/theme';
import Button from './shared/Button';
import Input from './shared/Input';

export interface RampOSKYCProps extends KYCCallbacks {
  apiKey: string;
  userId?: string;
  level?: KYCLevel;
  theme?: WidgetTheme;
  environment?: 'sandbox' | 'production';
}

type KYCStep = 'intro' | 'personal-info' | 'document-upload' | 'selfie' | 'review' | 'submitting' | 'submitted' | 'approved' | 'rejected';

const RampOSKYC: React.FC<RampOSKYCProps> = ({
  apiKey,
  userId,
  level = 'basic',
  theme: themeProp,
  onSubmitted,
  onApproved,
  onRejected,
  onError,
  onClose,
  onReady,
}) => {
  const theme = resolveTheme(themeProp);
  const emitter = RampOSEventEmitter.getInstance();

  const [step, setStep] = useState<KYCStep>('intro');
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [dateOfBirth, setDateOfBirth] = useState('');
  const [nationality, setNationality] = useState('');
  const [documentType, setDocumentType] = useState<KYCDocument['type']>('national_id');
  const [documentUploaded, setDocumentUploaded] = useState(false);
  const [selfieUploaded, setSelfieUploaded] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    emitter.emit('KYC_READY');
    onReady?.();
  }, []);

  const handleClose = useCallback(() => {
    emitter.emit('KYC_CLOSE');
    onClose?.();
  }, [emitter, onClose]);

  const handlePersonalInfoNext = () => {
    if (!firstName || !lastName || !dateOfBirth) {
      setError('Please fill in all required fields');
      return;
    }
    setError(null);
    setStep('document-upload');
  };

  const handleDocumentUpload = () => {
    // Simulate file upload
    setDocumentUploaded(true);
    if (level === 'basic') {
      setStep('review');
    } else {
      setStep('selfie');
    }
  };

  const handleSelfieUpload = () => {
    setSelfieUploaded(true);
    setStep('review');
  };

  const handleSubmit = async () => {
    setStep('submitting');
    setError(null);
    try {
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 2000));

      const result: KYCResult = {
        userId: userId || `user_${Date.now().toString(36)}`,
        status: 'pending' as KYCStatus,
        level,
        verifiedAt: undefined,
      };

      setStep('submitted');
      emitter.emit('KYC_SUBMITTED', result);
      onSubmitted?.(result);

      // Simulate background verification (in sandbox, auto-approve after delay)
      setTimeout(() => {
        const approvedResult: KYCResult = {
          ...result,
          status: 'approved',
          verifiedAt: Date.now(),
          expiresAt: Date.now() + 365 * 24 * 60 * 60 * 1000,
        };
        setStep('approved');
        emitter.emit('KYC_APPROVED', approvedResult);
        onApproved?.(approvedResult);
      }, 3000);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'KYC submission failed';
      setError(message);
      setStep('intro');
      emitter.emit('KYC_ERROR', { message });
      onError?.(err instanceof Error ? err : new Error(message));
    }
  };

  const containerStyle: React.CSSProperties = {
    fontFamily: theme.fontFamily,
    padding: '24px',
    borderRadius: theme.borderRadius,
    backgroundColor: theme.backgroundColor,
    color: theme.textColor,
    boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1)',
    maxWidth: '420px',
    width: '100%',
  };

  const headerStyle: React.CSSProperties = {
    fontSize: '18px',
    fontWeight: 600,
    marginBottom: '20px',
    borderBottom: '1px solid #e5e7eb',
    paddingBottom: '12px',
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  };

  const errorBoxStyle: React.CSSProperties = {
    color: theme.errorColor,
    fontSize: '13px',
    padding: '8px 12px',
    backgroundColor: '#fee2e2',
    borderRadius: '6px',
    marginBottom: '12px',
  };

  const stepIndicatorStyle: React.CSSProperties = {
    display: 'flex',
    gap: '4px',
    marginBottom: '20px',
  };

  const steps = level === 'basic'
    ? ['Info', 'Document', 'Review']
    : ['Info', 'Document', 'Selfie', 'Review'];

  const currentStepIndex = (() => {
    switch (step) {
      case 'intro': return -1;
      case 'personal-info': return 0;
      case 'document-upload': return 1;
      case 'selfie': return 2;
      case 'review': return level === 'basic' ? 2 : 3;
      default: return -1;
    }
  })();

  const renderStepIndicator = () => (
    <div style={stepIndicatorStyle}>
      {steps.map((s, i) => (
        <div key={s} style={{ flex: 1, textAlign: 'center' }}>
          <div style={{
            height: '4px',
            borderRadius: '2px',
            backgroundColor: i <= currentStepIndex ? theme.primaryColor : '#e5e7eb',
            marginBottom: '4px',
            transition: 'background-color 0.2s',
          }} />
          <span style={{ fontSize: '11px', color: i <= currentStepIndex ? theme.primaryColor : '#9ca3af' }}>{s}</span>
        </div>
      ))}
    </div>
  );

  const renderIntro = () => (
    <div style={{ textAlign: 'center', padding: '16px 0' }}>
      <div style={{ fontSize: '32px', marginBottom: '12px', color: theme.primaryColor }}>ID</div>
      <h3 style={{ margin: '0 0 8px', color: '#111827' }}>Identity Verification</h3>
      <p style={{ color: '#6b7280', fontSize: '14px', marginBottom: '8px' }}>
        Level: <strong>{level.charAt(0).toUpperCase() + level.slice(1)}</strong>
      </p>
      <p style={{ color: '#6b7280', fontSize: '13px', marginBottom: '24px', lineHeight: '1.5' }}>
        We need to verify your identity to comply with regulations.
        This usually takes a few minutes.
      </p>
      <Button onClick={() => setStep('personal-info')} primaryColor={theme.primaryColor}>
        Start Verification
      </Button>
    </div>
  );

  const renderPersonalInfo = () => (
    <div>
      {renderStepIndicator()}
      <Input label="First Name *" value={firstName} onChange={e => setFirstName(e.target.value)} placeholder="John" />
      <Input label="Last Name *" value={lastName} onChange={e => setLastName(e.target.value)} placeholder="Doe" />
      <Input label="Date of Birth *" type="date" value={dateOfBirth} onChange={e => setDateOfBirth(e.target.value)} />
      <Input label="Nationality" value={nationality} onChange={e => setNationality(e.target.value)} placeholder="Vietnamese" />
      {error && <div style={errorBoxStyle}>{error}</div>}
      <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
        <Button variant="secondary" onClick={() => setStep('intro')} primaryColor={theme.primaryColor}>Back</Button>
        <Button onClick={handlePersonalInfoNext} primaryColor={theme.primaryColor}>Next</Button>
      </div>
    </div>
  );

  const renderDocumentUpload = () => (
    <div>
      {renderStepIndicator()}
      <div style={{ fontSize: '14px', fontWeight: 500, marginBottom: '12px', color: '#374151' }}>
        Upload Identity Document
      </div>
      <div style={{ marginBottom: '16px' }}>
        <label style={{ fontSize: '13px', fontWeight: 500, color: '#374151', display: 'block', marginBottom: '8px' }}>
          Document Type
        </label>
        <select
          value={documentType}
          onChange={e => setDocumentType(e.target.value as KYCDocument['type'])}
          style={{
            width: '100%',
            padding: '8px 12px',
            border: '1px solid #d1d5db',
            borderRadius: '6px',
            fontSize: '14px',
            backgroundColor: '#fff',
          }}
        >
          <option value="national_id">National ID Card</option>
          <option value="passport">Passport</option>
          <option value="drivers_license">Driver's License</option>
        </select>
      </div>
      <div
        onClick={handleDocumentUpload}
        style={{
          border: `2px dashed ${documentUploaded ? theme.successColor : '#d1d5db'}`,
          borderRadius: '8px',
          padding: '32px',
          textAlign: 'center',
          cursor: 'pointer',
          backgroundColor: documentUploaded ? '#f0fdf4' : '#fafafa',
          transition: 'all 0.2s',
          marginBottom: '16px',
        }}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => { if (e.key === 'Enter') handleDocumentUpload(); }}
      >
        <div style={{ fontSize: '24px', marginBottom: '8px' }}>{documentUploaded ? '&#10003;' : '+'}</div>
        <div style={{ fontSize: '14px', fontWeight: 500, color: documentUploaded ? theme.successColor : '#6b7280' }}>
          {documentUploaded ? 'Document uploaded' : 'Click to upload front of document'}
        </div>
        <div style={{ fontSize: '12px', color: '#9ca3af', marginTop: '4px' }}>PNG, JPG up to 10MB</div>
      </div>
      <div style={{ display: 'flex', gap: '8px' }}>
        <Button variant="secondary" onClick={() => setStep('personal-info')} primaryColor={theme.primaryColor}>Back</Button>
      </div>
    </div>
  );

  const renderSelfie = () => (
    <div>
      {renderStepIndicator()}
      <div style={{ fontSize: '14px', fontWeight: 500, marginBottom: '12px', color: '#374151' }}>
        Take a Selfie
      </div>
      <p style={{ color: '#6b7280', fontSize: '13px', marginBottom: '16px' }}>
        Please take a clear photo of your face. Make sure your face is well-lit and fully visible.
      </p>
      <div
        onClick={handleSelfieUpload}
        style={{
          border: `2px dashed ${selfieUploaded ? theme.successColor : '#d1d5db'}`,
          borderRadius: '8px',
          padding: '32px',
          textAlign: 'center',
          cursor: 'pointer',
          backgroundColor: selfieUploaded ? '#f0fdf4' : '#fafafa',
          marginBottom: '16px',
        }}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => { if (e.key === 'Enter') handleSelfieUpload(); }}
      >
        <div style={{ fontSize: '24px', marginBottom: '8px' }}>{selfieUploaded ? '&#10003;' : '+'}</div>
        <div style={{ fontSize: '14px', fontWeight: 500, color: selfieUploaded ? theme.successColor : '#6b7280' }}>
          {selfieUploaded ? 'Selfie captured' : 'Click to take selfie'}
        </div>
      </div>
      <Button variant="secondary" onClick={() => setStep('document-upload')} primaryColor={theme.primaryColor}>Back</Button>
    </div>
  );

  const renderReview = () => (
    <div>
      {renderStepIndicator()}
      <div style={{ fontSize: '14px', fontWeight: 500, marginBottom: '16px', color: '#374151' }}>
        Review Your Information
      </div>
      <div style={{ backgroundColor: '#f9fafb', borderRadius: '8px', padding: '16px', marginBottom: '16px', fontSize: '14px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
          <span style={{ color: '#6b7280' }}>Name</span>
          <span style={{ fontWeight: 500 }}>{firstName} {lastName}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
          <span style={{ color: '#6b7280' }}>Date of Birth</span>
          <span style={{ fontWeight: 500 }}>{dateOfBirth}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
          <span style={{ color: '#6b7280' }}>Document</span>
          <span style={{ fontWeight: 500 }}>{documentType.replace('_', ' ')}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span style={{ color: '#6b7280' }}>Level</span>
          <span style={{ fontWeight: 500 }}>{level}</span>
        </div>
      </div>
      <Button onClick={handleSubmit} primaryColor={theme.primaryColor}>Submit for Verification</Button>
      <div style={{ marginTop: '8px' }}>
        <Button variant="secondary" onClick={() => setStep('document-upload')} primaryColor={theme.primaryColor}>Back</Button>
      </div>
    </div>
  );

  const renderSubmitting = () => (
    <div style={{ textAlign: 'center', padding: '24px 0' }}>
      <div style={{
        width: '44px', height: '44px',
        border: `3px solid ${theme.primaryColor}`,
        borderTopColor: 'transparent',
        borderRadius: '50%',
        margin: '0 auto 16px',
        animation: 'rampos-spin 0.8s linear infinite',
      }} />
      <div style={{ fontWeight: 500, color: '#374151' }}>Submitting your documents...</div>
      <style>{`@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }`}</style>
    </div>
  );

  const renderSubmitted = () => (
    <div style={{ textAlign: 'center', padding: '16px 0' }}>
      <div style={{
        width: '44px', height: '44px',
        border: `3px solid ${theme.primaryColor}`,
        borderTopColor: 'transparent',
        borderRadius: '50%',
        margin: '0 auto 16px',
        animation: 'rampos-spin 0.8s linear infinite',
      }} />
      <h3 style={{ margin: '0 0 8px', color: '#111827' }}>Verification In Progress</h3>
      <p style={{ color: '#6b7280', fontSize: '14px' }}>
        Your documents are being reviewed. This usually takes a few minutes.
      </p>
      <style>{`@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }`}</style>
    </div>
  );

  const renderApproved = () => (
    <div style={{ textAlign: 'center', padding: '16px 0' }}>
      <div style={{ color: theme.successColor, fontSize: '48px', marginBottom: '8px' }}>&#10003;</div>
      <h3 style={{ margin: '0 0 8px', color: '#111827' }}>Verification Complete</h3>
      <p style={{ color: '#6b7280', fontSize: '14px', marginBottom: '20px' }}>
        Your identity has been verified successfully.
      </p>
      <Button onClick={handleClose} primaryColor={theme.primaryColor}>Done</Button>
    </div>
  );

  const renderRejected = () => (
    <div style={{ textAlign: 'center', padding: '16px 0' }}>
      <div style={{ color: theme.errorColor, fontSize: '48px', marginBottom: '8px' }}>&#10007;</div>
      <h3 style={{ margin: '0 0 8px', color: '#111827' }}>Verification Failed</h3>
      <p style={{ color: '#6b7280', fontSize: '14px', marginBottom: '20px' }}>
        We were unable to verify your identity. Please try again with clearer documents.
      </p>
      <Button onClick={() => setStep('intro')} primaryColor={theme.primaryColor}>Try Again</Button>
      <div style={{ marginTop: '8px' }}>
        <Button variant="ghost" onClick={handleClose} primaryColor={theme.primaryColor}>Close</Button>
      </div>
    </div>
  );

  return (
    <div style={containerStyle} data-testid="rampos-kyc">
      <div style={headerStyle}>
        <span>RampOS KYC</span>
        <button
          onClick={handleClose}
          style={{ background: 'none', border: 'none', fontSize: '20px', cursor: 'pointer', color: '#9ca3af' }}
          aria-label="Close"
        >
          x
        </button>
      </div>

      {step === 'intro' && renderIntro()}
      {step === 'personal-info' && renderPersonalInfo()}
      {step === 'document-upload' && renderDocumentUpload()}
      {step === 'selfie' && renderSelfie()}
      {step === 'review' && renderReview()}
      {step === 'submitting' && renderSubmitting()}
      {step === 'submitted' && renderSubmitted()}
      {step === 'approved' && renderApproved()}
      {step === 'rejected' && renderRejected()}

      <div style={{ marginTop: '20px', textAlign: 'center', fontSize: '11px', color: '#9ca3af' }}>
        Powered by RampOS
      </div>
    </div>
  );
};

export default RampOSKYC;
