import React from 'react';
export interface ModalProps {
    open: boolean;
    onClose: () => void;
    title?: string;
    children: React.ReactNode;
    width?: string;
}
declare const Modal: React.FC<ModalProps>;
export default Modal;
