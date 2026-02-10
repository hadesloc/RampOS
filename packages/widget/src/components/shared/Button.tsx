import React from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost';
  fullWidth?: boolean;
  loading?: boolean;
  primaryColor?: string;
}

const Button: React.FC<ButtonProps> = ({
  variant = 'primary',
  fullWidth = true,
  loading = false,
  primaryColor = '#2563eb',
  children,
  disabled,
  style,
  ...rest
}) => {
  const baseStyle: React.CSSProperties = {
    border: 'none',
    borderRadius: '6px',
    padding: '10px 16px',
    fontSize: '14px',
    fontWeight: 500,
    cursor: disabled || loading ? 'not-allowed' : 'pointer',
    width: fullWidth ? '100%' : 'auto',
    transition: 'background-color 0.2s, opacity 0.2s',
    opacity: disabled || loading ? 0.6 : 1,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '8px',
  };

  const variants: Record<string, React.CSSProperties> = {
    primary: {
      backgroundColor: primaryColor,
      color: '#ffffff',
    },
    secondary: {
      backgroundColor: 'transparent',
      color: '#6b7280',
      border: '1px solid #d1d5db',
    },
    ghost: {
      backgroundColor: 'transparent',
      color: primaryColor,
    },
  };

  return (
    <button
      style={{ ...baseStyle, ...variants[variant], ...style }}
      disabled={disabled || loading}
      {...rest}
    >
      {loading && (
        <span style={{
          display: 'inline-block',
          width: '14px',
          height: '14px',
          border: '2px solid currentColor',
          borderTopColor: 'transparent',
          borderRadius: '50%',
          animation: 'rampos-spin 0.6s linear infinite',
        }} />
      )}
      {children}
    </button>
  );
};

export default Button;
