import { AxiosInstance } from 'axios';
import { z } from 'zod';

declare enum IntentType {
    PAYIN = "PAYIN",
    PAYOUT = "PAYOUT",
    TRADE = "TRADE"
}
declare const StateHistoryEntrySchema: z.ZodObject<{
    state: z.ZodString;
    timestamp: z.ZodString;
    reason: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    state: string;
    timestamp: string;
    reason?: string | undefined;
}, {
    state: string;
    timestamp: string;
    reason?: string | undefined;
}>;
type StateHistoryEntry = z.infer<typeof StateHistoryEntrySchema>;
declare const IntentSchema: z.ZodObject<{
    id: z.ZodString;
    userId: z.ZodOptional<z.ZodString>;
    intentType: z.ZodString;
    state: z.ZodString;
    amount: z.ZodString;
    currency: z.ZodString;
    actualAmount: z.ZodOptional<z.ZodString>;
    referenceCode: z.ZodOptional<z.ZodString>;
    bankTxId: z.ZodOptional<z.ZodString>;
    chainId: z.ZodOptional<z.ZodString>;
    txHash: z.ZodOptional<z.ZodString>;
    stateHistory: z.ZodOptional<z.ZodArray<z.ZodObject<{
        state: z.ZodString;
        timestamp: z.ZodString;
        reason: z.ZodOptional<z.ZodString>;
    }, "strip", z.ZodTypeAny, {
        state: string;
        timestamp: string;
        reason?: string | undefined;
    }, {
        state: string;
        timestamp: string;
        reason?: string | undefined;
    }>, "many">>;
    createdAt: z.ZodString;
    updatedAt: z.ZodString;
    expiresAt: z.ZodOptional<z.ZodString>;
    completedAt: z.ZodOptional<z.ZodString>;
    metadata: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
}, "strip", z.ZodTypeAny, {
    state: string;
    id: string;
    intentType: string;
    amount: string;
    currency: string;
    createdAt: string;
    updatedAt: string;
    userId?: string | undefined;
    actualAmount?: string | undefined;
    referenceCode?: string | undefined;
    bankTxId?: string | undefined;
    chainId?: string | undefined;
    txHash?: string | undefined;
    stateHistory?: {
        state: string;
        timestamp: string;
        reason?: string | undefined;
    }[] | undefined;
    expiresAt?: string | undefined;
    completedAt?: string | undefined;
    metadata?: Record<string, any> | undefined;
}, {
    state: string;
    id: string;
    intentType: string;
    amount: string;
    currency: string;
    createdAt: string;
    updatedAt: string;
    userId?: string | undefined;
    actualAmount?: string | undefined;
    referenceCode?: string | undefined;
    bankTxId?: string | undefined;
    chainId?: string | undefined;
    txHash?: string | undefined;
    stateHistory?: {
        state: string;
        timestamp: string;
        reason?: string | undefined;
    }[] | undefined;
    expiresAt?: string | undefined;
    completedAt?: string | undefined;
    metadata?: Record<string, any> | undefined;
}>;
type Intent = z.infer<typeof IntentSchema>;
declare const VirtualAccountSchema: z.ZodObject<{
    bank: z.ZodString;
    accountNumber: z.ZodString;
    accountName: z.ZodString;
}, "strip", z.ZodTypeAny, {
    bank: string;
    accountNumber: string;
    accountName: string;
}, {
    bank: string;
    accountNumber: string;
    accountName: string;
}>;
type VirtualAccount = z.infer<typeof VirtualAccountSchema>;
declare const BankAccountSchema: z.ZodObject<{
    bankCode: z.ZodString;
    accountNumber: z.ZodString;
    accountName: z.ZodString;
}, "strip", z.ZodTypeAny, {
    accountNumber: string;
    accountName: string;
    bankCode: string;
}, {
    accountNumber: string;
    accountName: string;
    bankCode: string;
}>;
type BankAccount = z.infer<typeof BankAccountSchema>;
declare const CreatePayinRequestSchema: z.ZodObject<{
    tenantId: z.ZodString;
    userId: z.ZodString;
    amountVnd: z.ZodNumber;
    railsProvider: z.ZodString;
    metadata: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
}, "strip", z.ZodTypeAny, {
    userId: string;
    tenantId: string;
    amountVnd: number;
    railsProvider: string;
    metadata?: Record<string, any> | undefined;
}, {
    userId: string;
    tenantId: string;
    amountVnd: number;
    railsProvider: string;
    metadata?: Record<string, any> | undefined;
}>;
type CreatePayinRequest = z.infer<typeof CreatePayinRequestSchema>;
declare const CreatePayinResponseSchema: z.ZodObject<{
    intentId: z.ZodString;
    referenceCode: z.ZodString;
    virtualAccount: z.ZodOptional<z.ZodObject<{
        bank: z.ZodString;
        accountNumber: z.ZodString;
        accountName: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        bank: string;
        accountNumber: string;
        accountName: string;
    }, {
        bank: string;
        accountNumber: string;
        accountName: string;
    }>>;
    expiresAt: z.ZodString;
    status: z.ZodString;
}, "strip", z.ZodTypeAny, {
    status: string;
    referenceCode: string;
    expiresAt: string;
    intentId: string;
    virtualAccount?: {
        bank: string;
        accountNumber: string;
        accountName: string;
    } | undefined;
}, {
    status: string;
    referenceCode: string;
    expiresAt: string;
    intentId: string;
    virtualAccount?: {
        bank: string;
        accountNumber: string;
        accountName: string;
    } | undefined;
}>;
type CreatePayinResponse = z.infer<typeof CreatePayinResponseSchema>;
declare const ConfirmPayinRequestSchema: z.ZodObject<{
    tenantId: z.ZodString;
    referenceCode: z.ZodString;
    status: z.ZodString;
    bankTxId: z.ZodString;
    amountVnd: z.ZodNumber;
    settledAt: z.ZodString;
    rawPayloadHash: z.ZodString;
}, "strip", z.ZodTypeAny, {
    status: string;
    referenceCode: string;
    bankTxId: string;
    tenantId: string;
    amountVnd: number;
    settledAt: string;
    rawPayloadHash: string;
}, {
    status: string;
    referenceCode: string;
    bankTxId: string;
    tenantId: string;
    amountVnd: number;
    settledAt: string;
    rawPayloadHash: string;
}>;
type ConfirmPayinRequest = z.infer<typeof ConfirmPayinRequestSchema>;
declare const ConfirmPayinResponseSchema: z.ZodObject<{
    intentId: z.ZodString;
    status: z.ZodString;
}, "strip", z.ZodTypeAny, {
    status: string;
    intentId: string;
}, {
    status: string;
    intentId: string;
}>;
type ConfirmPayinResponse = z.infer<typeof ConfirmPayinResponseSchema>;
declare const CreatePayoutRequestSchema: z.ZodObject<{
    tenantId: z.ZodString;
    userId: z.ZodString;
    amountVnd: z.ZodNumber;
    railsProvider: z.ZodString;
    bankAccount: z.ZodObject<{
        bankCode: z.ZodString;
        accountNumber: z.ZodString;
        accountName: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        accountNumber: string;
        accountName: string;
        bankCode: string;
    }, {
        accountNumber: string;
        accountName: string;
        bankCode: string;
    }>;
    metadata: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
}, "strip", z.ZodTypeAny, {
    userId: string;
    tenantId: string;
    amountVnd: number;
    railsProvider: string;
    bankAccount: {
        accountNumber: string;
        accountName: string;
        bankCode: string;
    };
    metadata?: Record<string, any> | undefined;
}, {
    userId: string;
    tenantId: string;
    amountVnd: number;
    railsProvider: string;
    bankAccount: {
        accountNumber: string;
        accountName: string;
        bankCode: string;
    };
    metadata?: Record<string, any> | undefined;
}>;
type CreatePayoutRequest = z.infer<typeof CreatePayoutRequestSchema>;
declare const CreatePayoutResponseSchema: z.ZodObject<{
    intentId: z.ZodString;
    status: z.ZodString;
}, "strip", z.ZodTypeAny, {
    status: string;
    intentId: string;
}, {
    status: string;
    intentId: string;
}>;
type CreatePayoutResponse = z.infer<typeof CreatePayoutResponseSchema>;
declare const CreatePayInSchema: z.ZodObject<{
    tenantId: z.ZodString;
    userId: z.ZodString;
    amountVnd: z.ZodNumber;
    railsProvider: z.ZodString;
    metadata: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
}, "strip", z.ZodTypeAny, {
    userId: string;
    tenantId: string;
    amountVnd: number;
    railsProvider: string;
    metadata?: Record<string, any> | undefined;
}, {
    userId: string;
    tenantId: string;
    amountVnd: number;
    railsProvider: string;
    metadata?: Record<string, any> | undefined;
}>;
type CreatePayInDto = CreatePayinRequest;
declare const CreatePayOutSchema: z.ZodObject<{
    tenantId: z.ZodString;
    userId: z.ZodString;
    amountVnd: z.ZodNumber;
    railsProvider: z.ZodString;
    bankAccount: z.ZodObject<{
        bankCode: z.ZodString;
        accountNumber: z.ZodString;
        accountName: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        accountNumber: string;
        accountName: string;
        bankCode: string;
    }, {
        accountNumber: string;
        accountName: string;
        bankCode: string;
    }>;
    metadata: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
}, "strip", z.ZodTypeAny, {
    userId: string;
    tenantId: string;
    amountVnd: number;
    railsProvider: string;
    bankAccount: {
        accountNumber: string;
        accountName: string;
        bankCode: string;
    };
    metadata?: Record<string, any> | undefined;
}, {
    userId: string;
    tenantId: string;
    amountVnd: number;
    railsProvider: string;
    bankAccount: {
        accountNumber: string;
        accountName: string;
        bankCode: string;
    };
    metadata?: Record<string, any> | undefined;
}>;
type CreatePayOutDto = CreatePayoutRequest;
declare const IntentFilterSchema: z.ZodObject<{
    userId: z.ZodOptional<z.ZodString>;
    intentType: z.ZodOptional<z.ZodString>;
    state: z.ZodOptional<z.ZodString>;
    limit: z.ZodOptional<z.ZodNumber>;
    offset: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    state?: string | undefined;
    userId?: string | undefined;
    intentType?: string | undefined;
    limit?: number | undefined;
    offset?: number | undefined;
}, {
    state?: string | undefined;
    userId?: string | undefined;
    intentType?: string | undefined;
    limit?: number | undefined;
    offset?: number | undefined;
}>;
type IntentFilters = z.infer<typeof IntentFilterSchema>;

declare class IntentService {
    private readonly httpClient;
    constructor(httpClient: AxiosInstance);
    /**
     * Create a new Pay-In intent.
     * @param data Pay-In data
     * @returns Created Intent
     */
    createPayIn(data: CreatePayinRequest): Promise<CreatePayinResponse>;
    /**
     * Confirm a Pay-In intent.
     * @param data Confirm Pay-In data
     * @returns Confirmation result
     */
    confirmPayIn(data: ConfirmPayinRequest): Promise<ConfirmPayinResponse>;
    /**
     * Create a new Pay-Out intent.
     * @param data Pay-Out data
     * @returns Created Intent
     */
    createPayOut(data: CreatePayoutRequest): Promise<CreatePayoutResponse>;
    /**
     * Get an intent by ID.
     * @param id Intent ID
     * @returns Intent
     */
    get(id: string): Promise<Intent>;
    /**
     * List intents with filters.
     * @param filters Filter criteria
     * @returns List of Intents
     */
    list(filters?: IntentFilters): Promise<Intent[]>;
}

declare const BalanceSchema: z.ZodObject<{
    accountType: z.ZodString;
    currency: z.ZodString;
    balance: z.ZodString;
}, "strip", z.ZodTypeAny, {
    currency: string;
    accountType: string;
    balance: string;
}, {
    currency: string;
    accountType: string;
    balance: string;
}>;
type Balance = z.infer<typeof BalanceSchema>;
declare const UserBalancesResponseSchema: z.ZodObject<{
    balances: z.ZodArray<z.ZodObject<{
        accountType: z.ZodString;
        currency: z.ZodString;
        balance: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        currency: string;
        accountType: string;
        balance: string;
    }, {
        currency: string;
        accountType: string;
        balance: string;
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    balances: {
        currency: string;
        accountType: string;
        balance: string;
    }[];
}, {
    balances: {
        currency: string;
        accountType: string;
        balance: string;
    }[];
}>;
type UserBalancesResponse = z.infer<typeof UserBalancesResponseSchema>;
declare const UserBalanceSchema: z.ZodObject<{
    accountType: z.ZodString;
    currency: z.ZodString;
    balance: z.ZodString;
}, "strip", z.ZodTypeAny, {
    currency: string;
    accountType: string;
    balance: string;
}, {
    currency: string;
    accountType: string;
    balance: string;
}>;
type UserBalance = Balance;
declare enum KycStatus {
    NONE = "NONE",
    PENDING = "PENDING",
    VERIFIED = "VERIFIED",
    REJECTED = "REJECTED"
}
declare const UserKycStatusSchema: z.ZodObject<{
    userId: z.ZodString;
    status: z.ZodNativeEnum<typeof KycStatus>;
    updatedAt: z.ZodString;
}, "strip", z.ZodTypeAny, {
    status: KycStatus;
    userId: string;
    updatedAt: string;
}, {
    status: KycStatus;
    userId: string;
    updatedAt: string;
}>;
type UserKycStatus = z.infer<typeof UserKycStatusSchema>;

declare class UserService {
    private readonly httpClient;
    constructor(httpClient: AxiosInstance);
    /**
     * Get user balances.
     * @param userId User ID
     * @returns List of User Balances
     */
    getBalances(userId: string): Promise<UserBalance[]>;
    /**
     * Get user KYC status.
     * @param tenantId Tenant ID
     * @param userId User ID
     * @returns User KYC Status
     */
    getKycStatus(tenantId: string, userId: string): Promise<UserKycStatus>;
}

declare enum LedgerEntryType {
    CREDIT = "CREDIT",
    DEBIT = "DEBIT"
}
declare const LedgerEntrySchema: z.ZodObject<{
    id: z.ZodString;
    tenantId: z.ZodString;
    transactionId: z.ZodString;
    type: z.ZodNativeEnum<typeof LedgerEntryType>;
    amount: z.ZodString;
    currency: z.ZodString;
    balanceAfter: z.ZodString;
    referenceId: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
    createdAt: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: LedgerEntryType;
    id: string;
    amount: string;
    currency: string;
    createdAt: string;
    tenantId: string;
    transactionId: string;
    balanceAfter: string;
    referenceId?: string | undefined;
    description?: string | undefined;
}, {
    type: LedgerEntryType;
    id: string;
    amount: string;
    currency: string;
    createdAt: string;
    tenantId: string;
    transactionId: string;
    balanceAfter: string;
    referenceId?: string | undefined;
    description?: string | undefined;
}>;
type LedgerEntry = z.infer<typeof LedgerEntrySchema>;
declare const LedgerFilterSchema: z.ZodObject<{
    transactionId: z.ZodOptional<z.ZodString>;
    referenceId: z.ZodOptional<z.ZodString>;
    startDate: z.ZodOptional<z.ZodString>;
    endDate: z.ZodOptional<z.ZodString>;
    limit: z.ZodOptional<z.ZodNumber>;
    offset: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    limit?: number | undefined;
    offset?: number | undefined;
    transactionId?: string | undefined;
    referenceId?: string | undefined;
    startDate?: string | undefined;
    endDate?: string | undefined;
}, {
    limit?: number | undefined;
    offset?: number | undefined;
    transactionId?: string | undefined;
    referenceId?: string | undefined;
    startDate?: string | undefined;
    endDate?: string | undefined;
}>;
type LedgerFilters = z.infer<typeof LedgerFilterSchema>;

declare class LedgerService {
    private readonly httpClient;
    constructor(httpClient: AxiosInstance);
    /**
     * Get ledger entries with filters.
     * @param filters Filter criteria
     * @returns List of Ledger Entries
     */
    getEntries(filters?: LedgerFilters): Promise<LedgerEntry[]>;
}

declare const CreateAccountParamsSchema: z.ZodObject<{
    tenantId: z.ZodString;
    userId: z.ZodString;
    ownerAddress: z.ZodString;
}, "strip", z.ZodTypeAny, {
    userId: string;
    tenantId: string;
    ownerAddress: string;
}, {
    userId: string;
    tenantId: string;
    ownerAddress: string;
}>;
type CreateAccountParams = z.infer<typeof CreateAccountParamsSchema>;
declare const CreateAccountResponseSchema: z.ZodObject<{
    address: z.ZodString;
    owner: z.ZodString;
    accountType: z.ZodString;
    isDeployed: z.ZodBoolean;
    chainId: z.ZodNumber;
    entryPoint: z.ZodString;
}, "strip", z.ZodTypeAny, {
    chainId: number;
    accountType: string;
    address: string;
    owner: string;
    isDeployed: boolean;
    entryPoint: string;
}, {
    chainId: number;
    accountType: string;
    address: string;
    owner: string;
    isDeployed: boolean;
    entryPoint: string;
}>;
type CreateAccountResponse = z.infer<typeof CreateAccountResponseSchema>;
declare const GetAccountResponseSchema: z.ZodObject<{
    address: z.ZodString;
    owner: z.ZodString;
    isDeployed: z.ZodBoolean;
    chainId: z.ZodNumber;
    entryPoint: z.ZodString;
    accountType: z.ZodString;
}, "strip", z.ZodTypeAny, {
    chainId: number;
    accountType: string;
    address: string;
    owner: string;
    isDeployed: boolean;
    entryPoint: string;
}, {
    chainId: number;
    accountType: string;
    address: string;
    owner: string;
    isDeployed: boolean;
    entryPoint: string;
}>;
type GetAccountResponse = z.infer<typeof GetAccountResponseSchema>;
declare const SmartAccountSchema: z.ZodObject<{
    address: z.ZodString;
    owner: z.ZodString;
    isDeployed: z.ZodBoolean;
    chainId: z.ZodNumber;
    entryPoint: z.ZodString;
    accountType: z.ZodString;
}, "strip", z.ZodTypeAny, {
    chainId: number;
    accountType: string;
    address: string;
    owner: string;
    isDeployed: boolean;
    entryPoint: string;
}, {
    chainId: number;
    accountType: string;
    address: string;
    owner: string;
    isDeployed: boolean;
    entryPoint: string;
}>;
type SmartAccount = GetAccountResponse;
declare const UserOperationSchema: z.ZodObject<{
    sender: z.ZodString;
    nonce: z.ZodString;
    initCode: z.ZodOptional<z.ZodString>;
    callData: z.ZodString;
    callGasLimit: z.ZodString;
    verificationGasLimit: z.ZodString;
    preVerificationGas: z.ZodString;
    maxFeePerGas: z.ZodString;
    maxPriorityFeePerGas: z.ZodString;
    paymasterAndData: z.ZodOptional<z.ZodString>;
    signature: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    sender: string;
    nonce: string;
    callData: string;
    callGasLimit: string;
    verificationGasLimit: string;
    preVerificationGas: string;
    maxFeePerGas: string;
    maxPriorityFeePerGas: string;
    initCode?: string | undefined;
    paymasterAndData?: string | undefined;
    signature?: string | undefined;
}, {
    sender: string;
    nonce: string;
    callData: string;
    callGasLimit: string;
    verificationGasLimit: string;
    preVerificationGas: string;
    maxFeePerGas: string;
    maxPriorityFeePerGas: string;
    initCode?: string | undefined;
    paymasterAndData?: string | undefined;
    signature?: string | undefined;
}>;
type UserOperation = z.infer<typeof UserOperationSchema>;
declare const SendUserOperationRequestSchema: z.ZodObject<{
    tenantId: z.ZodString;
    userOperation: z.ZodObject<{
        sender: z.ZodString;
        nonce: z.ZodString;
        initCode: z.ZodOptional<z.ZodString>;
        callData: z.ZodString;
        callGasLimit: z.ZodString;
        verificationGasLimit: z.ZodString;
        preVerificationGas: z.ZodString;
        maxFeePerGas: z.ZodString;
        maxPriorityFeePerGas: z.ZodString;
        paymasterAndData: z.ZodOptional<z.ZodString>;
        signature: z.ZodOptional<z.ZodString>;
    }, "strip", z.ZodTypeAny, {
        sender: string;
        nonce: string;
        callData: string;
        callGasLimit: string;
        verificationGasLimit: string;
        preVerificationGas: string;
        maxFeePerGas: string;
        maxPriorityFeePerGas: string;
        initCode?: string | undefined;
        paymasterAndData?: string | undefined;
        signature?: string | undefined;
    }, {
        sender: string;
        nonce: string;
        callData: string;
        callGasLimit: string;
        verificationGasLimit: string;
        preVerificationGas: string;
        maxFeePerGas: string;
        maxPriorityFeePerGas: string;
        initCode?: string | undefined;
        paymasterAndData?: string | undefined;
        signature?: string | undefined;
    }>;
    sponsor: z.ZodOptional<z.ZodBoolean>;
}, "strip", z.ZodTypeAny, {
    tenantId: string;
    userOperation: {
        sender: string;
        nonce: string;
        callData: string;
        callGasLimit: string;
        verificationGasLimit: string;
        preVerificationGas: string;
        maxFeePerGas: string;
        maxPriorityFeePerGas: string;
        initCode?: string | undefined;
        paymasterAndData?: string | undefined;
        signature?: string | undefined;
    };
    sponsor?: boolean | undefined;
}, {
    tenantId: string;
    userOperation: {
        sender: string;
        nonce: string;
        callData: string;
        callGasLimit: string;
        verificationGasLimit: string;
        preVerificationGas: string;
        maxFeePerGas: string;
        maxPriorityFeePerGas: string;
        initCode?: string | undefined;
        paymasterAndData?: string | undefined;
        signature?: string | undefined;
    };
    sponsor?: boolean | undefined;
}>;
type SendUserOperationRequest = z.infer<typeof SendUserOperationRequestSchema>;
declare const SendUserOperationResponseSchema: z.ZodObject<{
    userOpHash: z.ZodString;
    sender: z.ZodString;
    nonce: z.ZodString;
    status: z.ZodString;
    sponsored: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    status: string;
    sender: string;
    nonce: string;
    userOpHash: string;
    sponsored: boolean;
}, {
    status: string;
    sender: string;
    nonce: string;
    userOpHash: string;
    sponsored: boolean;
}>;
type SendUserOperationResponse = z.infer<typeof SendUserOperationResponseSchema>;
declare const EstimateGasRequestSchema: z.ZodObject<{
    tenantId: z.ZodString;
    userOperation: z.ZodObject<{
        sender: z.ZodString;
        nonce: z.ZodString;
        initCode: z.ZodOptional<z.ZodString>;
        callData: z.ZodString;
        callGasLimit: z.ZodString;
        verificationGasLimit: z.ZodString;
        preVerificationGas: z.ZodString;
        maxFeePerGas: z.ZodString;
        maxPriorityFeePerGas: z.ZodString;
        paymasterAndData: z.ZodOptional<z.ZodString>;
        signature: z.ZodOptional<z.ZodString>;
    }, "strip", z.ZodTypeAny, {
        sender: string;
        nonce: string;
        callData: string;
        callGasLimit: string;
        verificationGasLimit: string;
        preVerificationGas: string;
        maxFeePerGas: string;
        maxPriorityFeePerGas: string;
        initCode?: string | undefined;
        paymasterAndData?: string | undefined;
        signature?: string | undefined;
    }, {
        sender: string;
        nonce: string;
        callData: string;
        callGasLimit: string;
        verificationGasLimit: string;
        preVerificationGas: string;
        maxFeePerGas: string;
        maxPriorityFeePerGas: string;
        initCode?: string | undefined;
        paymasterAndData?: string | undefined;
        signature?: string | undefined;
    }>;
}, "strip", z.ZodTypeAny, {
    tenantId: string;
    userOperation: {
        sender: string;
        nonce: string;
        callData: string;
        callGasLimit: string;
        verificationGasLimit: string;
        preVerificationGas: string;
        maxFeePerGas: string;
        maxPriorityFeePerGas: string;
        initCode?: string | undefined;
        paymasterAndData?: string | undefined;
        signature?: string | undefined;
    };
}, {
    tenantId: string;
    userOperation: {
        sender: string;
        nonce: string;
        callData: string;
        callGasLimit: string;
        verificationGasLimit: string;
        preVerificationGas: string;
        maxFeePerGas: string;
        maxPriorityFeePerGas: string;
        initCode?: string | undefined;
        paymasterAndData?: string | undefined;
        signature?: string | undefined;
    };
}>;
type EstimateGasRequest = z.infer<typeof EstimateGasRequestSchema>;
declare const GasEstimateSchema: z.ZodObject<{
    preVerificationGas: z.ZodString;
    verificationGasLimit: z.ZodString;
    callGasLimit: z.ZodString;
    maxFeePerGas: z.ZodString;
    maxPriorityFeePerGas: z.ZodString;
}, "strip", z.ZodTypeAny, {
    callGasLimit: string;
    verificationGasLimit: string;
    preVerificationGas: string;
    maxFeePerGas: string;
    maxPriorityFeePerGas: string;
}, {
    callGasLimit: string;
    verificationGasLimit: string;
    preVerificationGas: string;
    maxFeePerGas: string;
    maxPriorityFeePerGas: string;
}>;
type GasEstimate = z.infer<typeof GasEstimateSchema>;
declare const UserOpReceiptSchema: z.ZodObject<{
    userOpHash: z.ZodString;
    sender: z.ZodString;
    nonce: z.ZodString;
    success: z.ZodBoolean;
    actualGasCost: z.ZodString;
    actualGasUsed: z.ZodString;
    paymaster: z.ZodOptional<z.ZodString>;
    transactionHash: z.ZodString;
    blockHash: z.ZodString;
    blockNumber: z.ZodString;
}, "strip", z.ZodTypeAny, {
    sender: string;
    nonce: string;
    userOpHash: string;
    success: boolean;
    actualGasCost: string;
    actualGasUsed: string;
    transactionHash: string;
    blockHash: string;
    blockNumber: string;
    paymaster?: string | undefined;
}, {
    sender: string;
    nonce: string;
    userOpHash: string;
    success: boolean;
    actualGasCost: string;
    actualGasUsed: string;
    transactionHash: string;
    blockHash: string;
    blockNumber: string;
    paymaster?: string | undefined;
}>;
type UserOpReceipt = z.infer<typeof UserOpReceiptSchema>;
declare const SessionKeySchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    publicKey: z.ZodString;
    permissions: z.ZodArray<z.ZodString, "many">;
    validUntil: z.ZodNumber;
    validAfter: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    publicKey: string;
    permissions: string[];
    validUntil: number;
    id?: string | undefined;
    validAfter?: number | undefined;
}, {
    publicKey: string;
    permissions: string[];
    validUntil: number;
    id?: string | undefined;
    validAfter?: number | undefined;
}>;
type SessionKey = z.infer<typeof SessionKeySchema>;
declare const AddSessionKeyParamsSchema: z.ZodObject<{
    accountAddress: z.ZodString;
    sessionKey: z.ZodObject<{
        id: z.ZodOptional<z.ZodString>;
        publicKey: z.ZodString;
        permissions: z.ZodArray<z.ZodString, "many">;
        validUntil: z.ZodNumber;
        validAfter: z.ZodOptional<z.ZodNumber>;
    }, "strip", z.ZodTypeAny, {
        publicKey: string;
        permissions: string[];
        validUntil: number;
        id?: string | undefined;
        validAfter?: number | undefined;
    }, {
        publicKey: string;
        permissions: string[];
        validUntil: number;
        id?: string | undefined;
        validAfter?: number | undefined;
    }>;
}, "strip", z.ZodTypeAny, {
    accountAddress: string;
    sessionKey: {
        publicKey: string;
        permissions: string[];
        validUntil: number;
        id?: string | undefined;
        validAfter?: number | undefined;
    };
}, {
    accountAddress: string;
    sessionKey: {
        publicKey: string;
        permissions: string[];
        validUntil: number;
        id?: string | undefined;
        validAfter?: number | undefined;
    };
}>;
type AddSessionKeyParams = z.infer<typeof AddSessionKeyParamsSchema>;
declare const RemoveSessionKeyParamsSchema: z.ZodObject<{
    accountAddress: z.ZodString;
    keyId: z.ZodString;
}, "strip", z.ZodTypeAny, {
    accountAddress: string;
    keyId: string;
}, {
    accountAddress: string;
    keyId: string;
}>;
type RemoveSessionKeyParams = z.infer<typeof RemoveSessionKeyParamsSchema>;

declare class AAService {
    private readonly httpClient;
    constructor(httpClient: AxiosInstance);
    /**
     * Create a smart account for a user.
     * @param params Create Account Params
     * @returns Smart Account Info
     */
    createSmartAccount(params: CreateAccountParams): Promise<CreateAccountResponse>;
    /**
     * Get smart account info for a user.
     * @param address Smart Account Address
     * @returns Smart Account Info
     */
    getSmartAccount(address: string): Promise<SmartAccount>;
    /**
     * Add a session key to an account.
     * @param params Add Session Key Params
     * @returns Void (throws on error)
     */
    addSessionKey(params: AddSessionKeyParams): Promise<void>;
    /**
     * Remove a session key from an account.
     * @param params Remove Session Key Params
     * @returns Void (throws on error)
     */
    removeSessionKey(params: RemoveSessionKeyParams): Promise<void>;
    /**
     * Send a user operation.
     * @param params User Operation Params
     * @returns User Operation Receipt
     */
    sendUserOperation(params: SendUserOperationRequest): Promise<SendUserOperationResponse>;
    /**
     * Estimate gas for a user operation.
     * @param params User Operation Params
     * @returns Gas Estimate
     */
    estimateGas(params: EstimateGasRequest): Promise<GasEstimate>;
    /**
     * Get a user operation by hash.
     */
    getUserOperation(hash: string): Promise<UserOperation>;
    /**
     * Get a user operation receipt by hash.
     */
    getUserOperationReceipt(hash: string): Promise<UserOpReceipt>;
}

declare class WebhookVerifier {
    /**
     * Verifies the signature of a webhook payload.
     *
     * @param payload - The raw request body as a string.
     * @param signature - The signature header sent by RampOS (e.g., X-RampOS-Signature).
     * @param secret - The webhook signing secret provided by RampOS.
     * @returns True if the signature is valid, false otherwise.
     * @throws Error if any parameter is missing.
     */
    verify(payload: string, signature: string, secret: string): boolean;
}

interface RetryConfig {
    maxRetries?: number;
    baseDelay?: number;
}
declare function withRetry<T>(fn: () => Promise<T>, maxRetries?: number, baseDelay?: number): Promise<T>;

/**
 * API Error structure returned by the RampOS API
 */
interface ApiError {
    code: string;
    message: string;
    details?: Record<string, unknown>;
}
/**
 * Standard API response wrapper
 */
interface ApiResponse<T> {
    data: T;
    success: boolean;
    error?: ApiError;
    meta?: {
        total?: number;
        page?: number;
        limit?: number;
    };
}
/**
 * Paginated list response
 */
interface PaginatedResponse<T> extends ApiResponse<T[]> {
    meta: {
        total: number;
        page: number;
        limit: number;
        hasMore?: boolean;
    };
}
interface RampOSConfig {
    baseURL?: string;
    apiKey: string;
    apiSecret: string;
    tenantId?: string;
    timeout?: number;
    retry?: RetryConfig;
}

declare class RampOSClient {
    private readonly httpClient;
    readonly intents: IntentService;
    readonly users: UserService;
    readonly ledger: LedgerService;
    readonly aa: AAService;
    readonly webhooks: WebhookVerifier;
    constructor(config: RampOSConfig);
}

export { AAService, type AddSessionKeyParams, AddSessionKeyParamsSchema, type ApiError, type ApiResponse, type Balance, BalanceSchema, type BankAccount, BankAccountSchema, type ConfirmPayinRequest, ConfirmPayinRequestSchema, type ConfirmPayinResponse, ConfirmPayinResponseSchema, type CreateAccountParams, CreateAccountParamsSchema, type CreateAccountResponse, CreateAccountResponseSchema, type CreatePayInDto, CreatePayInSchema, type CreatePayOutDto, CreatePayOutSchema, type CreatePayinRequest, CreatePayinRequestSchema, type CreatePayinResponse, CreatePayinResponseSchema, type CreatePayoutRequest, CreatePayoutRequestSchema, type CreatePayoutResponse, CreatePayoutResponseSchema, type EstimateGasRequest, EstimateGasRequestSchema, type GasEstimate, GasEstimateSchema, type GetAccountResponse, GetAccountResponseSchema, type Intent, IntentFilterSchema, type IntentFilters, IntentSchema, IntentService, IntentType, KycStatus, type LedgerEntry, LedgerEntrySchema, LedgerEntryType, LedgerFilterSchema, type LedgerFilters, LedgerService, type PaginatedResponse, RampOSClient, type RampOSConfig, type RemoveSessionKeyParams, RemoveSessionKeyParamsSchema, type RetryConfig, type SendUserOperationRequest, SendUserOperationRequestSchema, type SendUserOperationResponse, SendUserOperationResponseSchema, type SessionKey, SessionKeySchema, type SmartAccount, SmartAccountSchema, type StateHistoryEntry, StateHistoryEntrySchema, type UserBalance, UserBalanceSchema, type UserBalancesResponse, UserBalancesResponseSchema, type UserKycStatus, UserKycStatusSchema, type UserOpReceipt, UserOpReceiptSchema, type UserOperation, UserOperationSchema, UserService, type VirtualAccount, VirtualAccountSchema, WebhookVerifier, withRetry };
