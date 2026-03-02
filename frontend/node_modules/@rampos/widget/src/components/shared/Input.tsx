import React from 'react';

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  helpText?: string;
}

const Input: React.FC<InputProps> = ({
  label,
  error,
  helpText,
  style,
  id,
  ...rest
}) => {
  const inputId = id || `rampos-input-${label?.toLowerCase().replace(/\s+/g, '-')}`;

  return (
    <div style={{ marginBottom: '12px' }}>
      {label && (
        <label
          htmlFor={inputId}
          style={{
            display: 'block',
            fontSize: '14px',
            fontWeight: 500,
            marginBottom: '4px',
            color: '#374151',
          }}
        >
          {label}
        </label>
      )}
      <input
        id={inputId}
        style={{
          width: '100%',
          padding: '8px 12px',
          border: `1px solid ${error ? '#ef4444' : '#d1d5db'}`,
          borderRadius: '6px',
          fontSize: '14px',
          outline: 'none',
          boxSizing: 'border-box',
          transition: 'border-color 0.2s',
          ...style,
        }}
        {...rest}
      />
      {error && (
        <div style={{ color: '#ef4444', fontSize: '12px', marginTop: '4px' }}>
          {error}
        </div>
      )}
      {helpText && !error && (
        <div style={{ color: '#9ca3af', fontSize: '12px', marginTop: '4px' }}>
          {helpText}
        </div>
      )}
    </div>
  );
};

export default Input;
