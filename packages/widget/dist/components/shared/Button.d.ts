import React from 'react';
export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
    variant?: 'primary' | 'secondary' | 'ghost';
    fullWidth?: boolean;
    loading?: boolean;
    primaryColor?: string;
}
declare const Button: React.FC<ButtonProps>;
export default Button;
