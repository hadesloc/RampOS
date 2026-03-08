import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';

const {
  mockIsWebAuthnSupported,
  mockToAuthenticationOptions,
  mockToRegistrationOptions,
  mockStartAuthenticationWithAbort,
  mockStartRegistrationWithAbort,
  mockGetAuthenticationChallenge,
  mockVerifyAuthentication,
  mockGetRegistrationChallenge,
  mockVerifyRegistration,
  mockListPasskeys,
  mockDeletePasskey,
  mockRenamePasskey,
} = vi.hoisted(() => ({
  mockIsWebAuthnSupported: vi.fn(() => true),
  mockToAuthenticationOptions: vi.fn((c: { challenge: string }) => ({ challenge: c.challenge })),
  mockToRegistrationOptions: vi.fn((c: { challenge: string }) => ({ challenge: c.challenge })),
  mockStartAuthenticationWithAbort: vi.fn(),
  mockStartRegistrationWithAbort: vi.fn(),
  mockGetAuthenticationChallenge: vi.fn(),
  mockVerifyAuthentication: vi.fn(),
  mockGetRegistrationChallenge: vi.fn(),
  mockVerifyRegistration: vi.fn(),
  mockListPasskeys: vi.fn(),
  mockDeletePasskey: vi.fn(),
  mockRenamePasskey: vi.fn(),
}));

vi.mock('@/lib/webauthn', () => ({
  isWebAuthnSupported: mockIsWebAuthnSupported,
  toAuthenticationOptions: mockToAuthenticationOptions,
  toRegistrationOptions: mockToRegistrationOptions,
  startAuthenticationWithAbort: mockStartAuthenticationWithAbort,
  startRegistrationWithAbort: mockStartRegistrationWithAbort,
}));

vi.mock('@/lib/passkey-api', () => ({
  passkeyApi: {
    getAuthenticationChallenge: mockGetAuthenticationChallenge,
    verifyAuthentication: mockVerifyAuthentication,
    getRegistrationChallenge: mockGetRegistrationChallenge,
    verifyRegistration: mockVerifyRegistration,
    listPasskeys: mockListPasskeys,
    deletePasskey: mockDeletePasskey,
    renamePasskey: mockRenamePasskey,
  },
  PasskeyApiError: class PasskeyApiError extends Error {
    status: number;
    code: string;
    constructor(status: number, code: string, message: string) {
      super(message);
      this.name = 'PasskeyApiError';
      this.status = status;
      this.code = code;
    }
  },
}));

// Import components after mocks
import { PasskeyLogin } from '@/components/passkey/PasskeyLogin';
import { PasskeyRegister } from '@/components/passkey/PasskeyRegister';
import { PasskeyManagement } from '@/components/passkey/PasskeyManagement';

beforeEach(() => {
  vi.clearAllMocks();
  mockIsWebAuthnSupported.mockReturnValue(true);
});

// ========== PasskeyLogin Tests ==========

describe('PasskeyLogin', () => {
  it('renders login button', () => {
    render(<PasskeyLogin />);
    expect(
      screen.getByRole('button', { name: /dang nhap bang passkey/i }),
    ).toBeInTheDocument();
  });

  it('shows error when WebAuthn is not supported', async () => {
    mockIsWebAuthnSupported.mockReturnValue(false);
    const onError = vi.fn();

    render(<PasskeyLogin onError={onError} />);
    fireEvent.click(
      screen.getByRole('button', { name: /dang nhap bang passkey/i }),
    );

    await waitFor(() => {
      expect(screen.getByText(/khong ho tro webauthn/i)).toBeInTheDocument();
    });
    expect(onError).toHaveBeenCalled();
  });

  it('fetches challenge from backend and authenticates', async () => {
    const mockChallenge = {
      challenge: 'dGVzdC1jaGFsbGVuZ2U',
      rpId: 'localhost',
      timeout: 60000,
    };
    const mockCred = {
      id: 'cred-123',
      rawId: 'cmF3',
      type: 'public-key' as const,
      response: { clientDataJSON: 'Y2xpZW50', authenticatorData: 'YXV0aA', signature: 'c2ln' },
    };

    mockGetAuthenticationChallenge.mockResolvedValue(mockChallenge);
    mockStartAuthenticationWithAbort.mockResolvedValue(mockCred);
    mockVerifyAuthentication.mockResolvedValue({ userId: 'user-1', token: 'tok' });

    const onSuccess = vi.fn();
    render(<PasskeyLogin onSuccess={onSuccess} />);

    fireEvent.click(
      screen.getByRole('button', { name: /dang nhap bang passkey/i }),
    );

    await waitFor(() => {
      expect(onSuccess).toHaveBeenCalledWith(mockCred);
    });

    expect(mockGetAuthenticationChallenge).toHaveBeenCalled();
    expect(mockToAuthenticationOptions).toHaveBeenCalledWith(mockChallenge);
    expect(mockStartAuthenticationWithAbort).toHaveBeenCalled();
    expect(mockVerifyAuthentication).toHaveBeenCalledWith(mockCred);
  });

  it('handles NotAllowedError (user cancelled)', async () => {
    mockGetAuthenticationChallenge.mockResolvedValue({
      challenge: 'dGVzdA',
      rpId: 'localhost',
      timeout: 60000,
    });
    const domErr = new DOMException('User denied', 'NotAllowedError');
    mockStartAuthenticationWithAbort.mockRejectedValue(domErr);

    const onError = vi.fn();
    render(<PasskeyLogin onError={onError} />);

    fireEvent.click(
      screen.getByRole('button', { name: /dang nhap bang passkey/i }),
    );

    await waitFor(() => {
      expect(screen.getByText(/bi huy hoac het thoi gian/i)).toBeInTheDocument();
    });
    expect(onError).toHaveBeenCalled();
  });

  it('handles server API error', async () => {
    mockGetAuthenticationChallenge.mockRejectedValue(
      new Error('Server unavailable'),
    );

    const onError = vi.fn();
    render(<PasskeyLogin onError={onError} />);

    fireEvent.click(
      screen.getByRole('button', { name: /dang nhap bang passkey/i }),
    );

    await waitFor(() => {
      expect(onError).toHaveBeenCalled();
    });
  });

  it('shows loading state during authentication', async () => {
    let resolveChallenge!: (v: unknown) => void;
    mockGetAuthenticationChallenge.mockImplementation(
      () => new Promise((r) => { resolveChallenge = r; }),
    );

    render(<PasskeyLogin />);
    fireEvent.click(
      screen.getByRole('button', { name: /dang nhap bang passkey/i }),
    );

    await waitFor(() => {
      expect(screen.getByRole('button')).toBeDisabled();
    });

    resolveChallenge({
      challenge: 'dGVzdA',
      rpId: 'localhost',
      timeout: 60000,
    });
  });
});

// ========== PasskeyRegister Tests ==========

describe('PasskeyRegister', () => {
  it('renders register button and display name input', () => {
    render(<PasskeyRegister />);
    expect(
      screen.getByRole('button', { name: /dang ky passkey/i }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/ten hien thi/i)).toBeInTheDocument();
  });

  it('hides display name input when userName is provided', () => {
    render(<PasskeyRegister userName="Test User" />);
    expect(screen.queryByLabelText(/ten hien thi/i)).not.toBeInTheDocument();
  });

  it('shows error when display name is empty', async () => {
    const onError = vi.fn();
    render(<PasskeyRegister onError={onError} />);

    fireEvent.click(
      screen.getByRole('button', { name: /dang ky passkey/i }),
    );

    await waitFor(() => {
      expect(screen.getByText(/nhap ten hien thi/i)).toBeInTheDocument();
    });
    expect(onError).toHaveBeenCalled();
  });

  it('shows error when WebAuthn is not supported', async () => {
    mockIsWebAuthnSupported.mockReturnValue(false);
    const onError = vi.fn();

    render(<PasskeyRegister userName="Test" onError={onError} />);
    fireEvent.click(
      screen.getByRole('button', { name: /dang ky passkey/i }),
    );

    await waitFor(() => {
      expect(screen.getByText(/khong ho tro webauthn/i)).toBeInTheDocument();
    });
  });

  it('fetches challenge and registers with backend', async () => {
    const mockChallenge = {
      challenge: 'dGVzdA',
      rpId: 'localhost',
      rpName: 'RampOS',
      userId: 'dXNlcg',
      userName: 'test@test.com',
      userDisplayName: 'Test',
      timeout: 60000,
      attestation: 'none' as const,
      pubKeyCredParams: [{ alg: -7, type: 'public-key' as const }],
    };
    const mockCred = {
      id: 'cred-456',
      rawId: 'cmF3',
      type: 'public-key' as const,
      response: { clientDataJSON: 'Y2xpZW50', attestationObject: 'YXR0ZXN0' },
    };

    mockGetRegistrationChallenge.mockResolvedValue(mockChallenge);
    mockStartRegistrationWithAbort.mockResolvedValue(mockCred);
    mockVerifyRegistration.mockResolvedValue({
      credentialId: 'cred-456',
      userId: 'user-1',
    });

    const onSuccess = vi.fn();
    render(<PasskeyRegister userName="Test User" onSuccess={onSuccess} />);

    fireEvent.click(
      screen.getByRole('button', { name: /dang ky passkey/i }),
    );

    await waitFor(() => {
      expect(onSuccess).toHaveBeenCalledWith(mockCred);
    });

    expect(mockGetRegistrationChallenge).toHaveBeenCalledWith('Test User');
    expect(mockToRegistrationOptions).toHaveBeenCalledWith(mockChallenge);
    expect(mockVerifyRegistration).toHaveBeenCalledWith(mockCred);
  });

  it('handles InvalidStateError (duplicate passkey)', async () => {
    mockGetRegistrationChallenge.mockResolvedValue({
      challenge: 'dGVzdA',
      rpId: 'localhost',
      rpName: 'RampOS',
      userId: 'dXNlcg',
      userName: 'test@test.com',
      userDisplayName: 'Test',
      timeout: 60000,
      attestation: 'none',
      pubKeyCredParams: [{ alg: -7, type: 'public-key' }],
    });
    const domErr = new DOMException('Already registered', 'InvalidStateError');
    mockStartRegistrationWithAbort.mockRejectedValue(domErr);

    render(<PasskeyRegister userName="Test" />);
    fireEvent.click(
      screen.getByRole('button', { name: /dang ky passkey/i }),
    );

    await waitFor(() => {
      expect(screen.getByText(/da duoc dang ky truoc do/i)).toBeInTheDocument();
    });
  });

  it('shows success message after registration', async () => {
    mockGetRegistrationChallenge.mockResolvedValue({
      challenge: 'dGVzdA',
      rpId: 'localhost',
      rpName: 'RampOS',
      userId: 'dXNlcg',
      userName: 'test@test.com',
      userDisplayName: 'Test',
      timeout: 60000,
      attestation: 'none',
      pubKeyCredParams: [{ alg: -7, type: 'public-key' }],
    });
    mockStartRegistrationWithAbort.mockResolvedValue({
      id: 'c',
      rawId: 'r',
      type: 'public-key',
      response: { clientDataJSON: 'c', attestationObject: 'a' },
    });
    mockVerifyRegistration.mockResolvedValue({
      credentialId: 'c',
      userId: 'u',
    });

    render(<PasskeyRegister userName="Test" />);
    fireEvent.click(
      screen.getByRole('button', { name: /dang ky passkey/i }),
    );

    await waitFor(() => {
      expect(screen.getByText(/thanh cong/i)).toBeInTheDocument();
    });
  });
});

// ========== PasskeyManagement Tests ==========

describe('PasskeyManagement', () => {
  const mockPasskeys: PasskeyInfo[] = [
    {
      id: 'cred-1',
      name: 'iPhone Face ID',
      createdAt: '2025-01-15T10:00:00Z',
      lastUsedAt: '2025-06-01T14:30:00Z',
    },
    {
      id: 'cred-2',
      name: 'Windows Hello',
      createdAt: '2025-03-20T08:00:00Z',
      lastUsedAt: null,
    },
  ];

  // Need to import the type after mocks
  type PasskeyInfo = {
    id: string;
    name: string;
    createdAt: string;
    lastUsedAt: string | null;
  };

  it('loads and displays passkeys', async () => {
    mockListPasskeys.mockResolvedValue(mockPasskeys);

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(screen.getByText('iPhone Face ID')).toBeInTheDocument();
      expect(screen.getByText('Windows Hello')).toBeInTheDocument();
    });
  });

  it('shows empty state when no passkeys', async () => {
    mockListPasskeys.mockResolvedValue([]);

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(
        screen.getByText(/chua co passkey nao/i),
      ).toBeInTheDocument();
    });
  });

  it('shows error when loading fails', async () => {
    mockListPasskeys.mockRejectedValue(new Error('Network error'));

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(
        screen.getByText(/khong the tai danh sach/i),
      ).toBeInTheDocument();
    });
  });

  it('shows "Add passkey" button', async () => {
    mockListPasskeys.mockResolvedValue([]);

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: /them passkey/i }),
      ).toBeInTheDocument();
    });
  });

  it('switches to register view when "Add passkey" is clicked', async () => {
    mockListPasskeys.mockResolvedValue([]);

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: /them passkey/i }),
      ).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /them passkey/i }));

    expect(screen.getByText(/them passkey moi/i)).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /quay lai/i }),
    ).toBeInTheDocument();
  });

  it('shows rename input when rename is clicked', async () => {
    mockListPasskeys.mockResolvedValue(mockPasskeys);

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(screen.getByText('iPhone Face ID')).toBeInTheDocument();
    });

    const renameButtons = screen.getAllByRole('button', { name: /doi ten/i });
    fireEvent.click(renameButtons[0]);

    expect(screen.getByPlaceholderText(/ten moi/i)).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /luu/i }),
    ).toBeInTheDocument();
  });

  it('renames a passkey', async () => {
    mockListPasskeys.mockResolvedValue(mockPasskeys);
    mockRenamePasskey.mockResolvedValue({
      id: 'cred-1',
      name: 'My iPhone',
      createdAt: '2025-01-15T10:00:00Z',
      lastUsedAt: '2025-06-01T14:30:00Z',
    });

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(screen.getByText('iPhone Face ID')).toBeInTheDocument();
    });

    const renameButtons = screen.getAllByRole('button', { name: /doi ten/i });
    fireEvent.click(renameButtons[0]);

    const input = screen.getByPlaceholderText(/ten moi/i);
    fireEvent.change(input, { target: { value: 'My iPhone' } });
    fireEvent.click(screen.getByRole('button', { name: /luu/i }));

    await waitFor(() => {
      expect(mockRenamePasskey).toHaveBeenCalledWith('cred-1', 'My iPhone');
    });

    await waitFor(() => {
      expect(screen.getByText('My iPhone')).toBeInTheDocument();
    });
  });

  it('shows delete confirmation dialog', async () => {
    mockListPasskeys.mockResolvedValue(mockPasskeys);

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(screen.getByText('iPhone Face ID')).toBeInTheDocument();
    });

    const deleteButtons = screen.getAllByRole('button', { name: /^xoa$/i });
    fireEvent.click(deleteButtons[0]);

    expect(screen.getByText(/xac nhan xoa/i)).toBeInTheDocument();
  });

  it('deletes a passkey after confirmation', async () => {
    mockListPasskeys.mockResolvedValue(mockPasskeys);
    mockDeletePasskey.mockResolvedValue(undefined);

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(screen.getByText('iPhone Face ID')).toBeInTheDocument();
    });

    // Click the "Xoa" action button for first passkey
    const deleteButtons = screen.getAllByRole('button', { name: /^xoa$/i });
    fireEvent.click(deleteButtons[0]);

    // Confirm deletion
    await waitFor(() => {
      expect(screen.getByText(/xac nhan xoa/i)).toBeInTheDocument();
    });

    // Click the destructive "Xoa" button in confirmation
    const confirmDeleteButtons = screen.getAllByRole('button', { name: /^xoa$/i });
    // Find the destructive one (the confirm button in the dialog)
    const confirmBtn = confirmDeleteButtons.find((btn) =>
      btn.className.includes('destructive') ||
      btn.closest('.bg-destructive\\/10') !== null
    );
    if (confirmBtn) {
      fireEvent.click(confirmBtn);
    } else {
      // Fallback: click last Xoa button (the confirmation one)
      fireEvent.click(confirmDeleteButtons[confirmDeleteButtons.length - 1]);
    }

    await waitFor(() => {
      expect(mockDeletePasskey).toHaveBeenCalledWith('cred-1');
    });

    await waitFor(() => {
      expect(screen.queryByText('iPhone Face ID')).not.toBeInTheDocument();
    });
  });

  it('cancels delete when cancel button is clicked', async () => {
    mockListPasskeys.mockResolvedValue(mockPasskeys);

    render(<PasskeyManagement />);

    await waitFor(() => {
      expect(screen.getByText('iPhone Face ID')).toBeInTheDocument();
    });

    const deleteButtons = screen.getAllByRole('button', { name: /^xoa$/i });
    fireEvent.click(deleteButtons[0]);

    await waitFor(() => {
      expect(screen.getByText(/xac nhan xoa/i)).toBeInTheDocument();
    });

    // Click "Huy" to cancel
    const cancelButtons = screen.getAllByRole('button', { name: /huy/i });
    fireEvent.click(cancelButtons[cancelButtons.length - 1]);

    await waitFor(() => {
      expect(screen.queryByText(/xac nhan xoa/i)).not.toBeInTheDocument();
    });
    expect(mockDeletePasskey).not.toHaveBeenCalled();
  });
});
