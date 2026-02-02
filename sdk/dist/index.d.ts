import { AxiosInstance } from 'axios';
import { z } from 'zod';

declare enum IntentType {
    PAY_IN = "PAY_IN",
    PAY_OUT = "PAY_OUT",
    TRADE = "TRADE"
}
declare enum IntentStatus {
    CREATED = "CREATED",
    PENDING = "PENDING",
    COMPLETED = "COMPLETED",
    FAILED = "FAILED",
    CANCELLED = "CANCELLED"
}
declare const IntentSchema: z.ZodObject<{
    id: z.ZodString;
    tenantId: z.ZodString;
    type: z.ZodNativeEnum<typeof IntentType>;
    status: z.ZodNativeEnum<typeof IntentStatus>;
    amount: z.ZodString;
    currency: z.ZodString;
    bankAccount: z.ZodOptional<z.ZodString>;
    bankRef: z.ZodOptional<z.ZodString>;
    metadata: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
    createdAt: z.ZodString;
    updatedAt: z.ZodString;
}, "strip", z.ZodTypeAny, {
    id: string;
    tenantId: string;
    type: IntentType;
    status: IntentStatus;
    amount: string;
    currency: string;
    createdAt: string;
    updatedAt: string;
    bankAccount?: string | undefined;
    bankRef?: string | undefined;
    metadata?: Record<string, any> | undefined;
}, {
    id: string;
    tenantId: string;
    type: IntentType;
    status: IntentStatus;
    amount: string;
    currency: string;
    createdAt: string;
    updatedAt: string;
    bankAccount?: string | undefined;
    bankRef?: string | undefined;
    metadata?: Record<string, any> | undefined;
}>;
type Intent = z.infer<typeof IntentSchema>;
declare const CreatePayInSchema: z.ZodObject<{
    amount: z.ZodString;
    currency: z.ZodString;
    metadata: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
}, "strip", z.ZodTypeAny, {
    amount: string;
    currency: string;
    metadata?: Record<string, any> | undefined;
}, {
    amount: string;
    currency: string;
    metadata?: Record<string, any> | undefined;
}>;
type CreatePayInDto = z.infer<typeof CreatePayInSchema>;
declare const CreatePayOutSchema: z.ZodObject<{
    amount: z.ZodString;
    currency: z.ZodString;
    bankAccount: z.ZodString;
    metadata: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
}, "strip", z.ZodTypeAny, {
    amount: string;
    currency: string;
    bankAccount: string;
    metadata?: Record<string, any> | undefined;
}, {
    amount: string;
    currency: string;
    bankAccount: string;
    metadata?: Record<string, any> | undefined;
}>;
type CreatePayOutDto = z.infer<typeof CreatePayOutSchema>;
declare const IntentFilterSchema: z.ZodObject<{
    type: z.ZodOptional<z.ZodNativeEnum<typeof IntentType>>;
    status: z.ZodOptional<z.ZodNativeEnum<typeof IntentStatus>>;
    startDate: z.ZodOptional<z.ZodString>;
    endDate: z.ZodOptional<z.ZodString>;
    limit: z.ZodOptional<z.ZodNumber>;
    offset: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    type?: IntentType | undefined;
    status?: IntentStatus | undefined;
    startDate?: string | undefined;
    endDate?: string | undefined;
    limit?: number | undefined;
    offset?: number | undefined;
}, {
    type?: IntentType | undefined;
    status?: IntentStatus | undefined;
    startDate?: string | undefined;
    endDate?: string | undefined;
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
    createPayIn(data: CreatePayInDto): Promise<Intent>;
    /**
     * Confirm a Pay-In intent.
     * @param id Intent ID
     * @param bankRef Bank Reference Code
     * @returns Updated Intent
     */
    confirmPayIn(id: string, bankRef: string): Promise<Intent>;
    /**
     * Create a new Pay-Out intent.
     * @param data Pay-Out data
     * @returns Created Intent
     */
    createPayOut(data: CreatePayOutDto): Promise<Intent>;
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

declare const UserBalanceSchema: z.ZodObject<{
    currency: z.ZodString;
    amount: z.ZodString;
    locked: z.ZodString;
}, "strip", z.ZodTypeAny, {
    amount: string;
    currency: string;
    locked: string;
}, {
    amount: string;
    currency: string;
    locked: string;
}>;
type UserBalance = z.infer<typeof UserBalanceSchema>;
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
    updatedAt: string;
    userId: string;
}, {
    status: KycStatus;
    updatedAt: string;
    userId: string;
}>;
type UserKycStatus = z.infer<typeof UserKycStatusSchema>;

declare class UserService {
    private readonly httpClient;
    constructor(httpClient: AxiosInstance);
    /**
     * Get user balances.
     * @param tenantId Tenant ID
     * @param userId User ID
     * @returns List of User Balances
     */
    getBalances(tenantId: string, userId: string): Promise<UserBalance[]>;
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
    id: string;
    tenantId: string;
    type: LedgerEntryType;
    amount: string;
    currency: string;
    createdAt: string;
    transactionId: string;
    balanceAfter: string;
    referenceId?: string | undefined;
    description?: string | undefined;
}, {
    id: string;
    tenantId: string;
    type: LedgerEntryType;
    amount: string;
    currency: string;
    createdAt: string;
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
    startDate?: string | undefined;
    endDate?: string | undefined;
    limit?: number | undefined;
    offset?: number | undefined;
    transactionId?: string | undefined;
    referenceId?: string | undefined;
}, {
    startDate?: string | undefined;
    endDate?: string | undefined;
    limit?: number | undefined;
    offset?: number | undefined;
    transactionId?: string | undefined;
    referenceId?: string | undefined;
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

declare const SmartAccountSchema: z.ZodObject<{
    address: z.ZodString;
    owner: z.ZodString;
    factoryAddress: z.ZodString;
    deployed: z.ZodBoolean;
    balance: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    address: string;
    owner: string;
    factoryAddress: string;
    deployed: boolean;
    balance?: string | undefined;
}, {
    address: string;
    owner: string;
    factoryAddress: string;
    deployed: boolean;
    balance?: string | undefined;
}>;
type SmartAccount = z.infer<typeof SmartAccountSchema>;
declare const CreateAccountParamsSchema: z.ZodObject<{
    owner: z.ZodString;
    salt: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    owner: string;
    salt?: string | undefined;
}, {
    owner: string;
    salt?: string | undefined;
}>;
type CreateAccountParams = z.infer<typeof CreateAccountParamsSchema>;
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
declare const UserOperationSchema: z.ZodObject<{
    sender: z.ZodString;
    nonce: z.ZodString;
    initCode: z.ZodString;
    callData: z.ZodString;
    callGasLimit: z.ZodString;
    verificationGasLimit: z.ZodString;
    preVerificationGas: z.ZodString;
    maxFeePerGas: z.ZodString;
    maxPriorityFeePerGas: z.ZodString;
    paymasterAndData: z.ZodString;
    signature: z.ZodString;
}, "strip", z.ZodTypeAny, {
    sender: string;
    nonce: string;
    initCode: string;
    callData: string;
    callGasLimit: string;
    verificationGasLimit: string;
    preVerificationGas: string;
    maxFeePerGas: string;
    maxPriorityFeePerGas: string;
    paymasterAndData: string;
    signature: string;
}, {
    sender: string;
    nonce: string;
    initCode: string;
    callData: string;
    callGasLimit: string;
    verificationGasLimit: string;
    preVerificationGas: string;
    maxFeePerGas: string;
    maxPriorityFeePerGas: string;
    paymasterAndData: string;
    signature: string;
}>;
type UserOperation = z.infer<typeof UserOperationSchema>;
declare const UserOperationParamsSchema: z.ZodObject<{
    target: z.ZodString;
    value: z.ZodDefault<z.ZodString>;
    data: z.ZodDefault<z.ZodString>;
    sponsored: z.ZodOptional<z.ZodBoolean>;
    accountAddress: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    value: string;
    target: string;
    data: string;
    accountAddress?: string | undefined;
    sponsored?: boolean | undefined;
}, {
    target: string;
    value?: string | undefined;
    accountAddress?: string | undefined;
    data?: string | undefined;
    sponsored?: boolean | undefined;
}>;
type UserOperationParams = z.infer<typeof UserOperationParamsSchema>;
declare const GasEstimateSchema: z.ZodObject<{
    preVerificationGas: z.ZodString;
    verificationGas: z.ZodString;
    callGasLimit: z.ZodString;
    total: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    callGasLimit: string;
    preVerificationGas: string;
    verificationGas: string;
    total?: string | undefined;
}, {
    callGasLimit: string;
    preVerificationGas: string;
    verificationGas: string;
    total?: string | undefined;
}>;
type GasEstimate = z.infer<typeof GasEstimateSchema>;
declare const UserOpReceiptSchema: z.ZodObject<{
    userOpHash: z.ZodString;
    txHash: z.ZodOptional<z.ZodString>;
    success: z.ZodOptional<z.ZodBoolean>;
}, "strip", z.ZodTypeAny, {
    userOpHash: string;
    txHash?: string | undefined;
    success?: boolean | undefined;
}, {
    userOpHash: string;
    txHash?: string | undefined;
    success?: boolean | undefined;
}>;
type UserOpReceipt = z.infer<typeof UserOpReceiptSchema>;

declare class AAService {
    private readonly httpClient;
    constructor(httpClient: AxiosInstance);
    /**
     * Create a smart account for a user.
     * @param params Create Account Params
     * @returns Smart Account Info
     */
    createSmartAccount(params: CreateAccountParams): Promise<SmartAccount>;
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
    sendUserOperation(params: UserOperationParams): Promise<UserOpReceipt>;
    /**
     * Estimate gas for a user operation.
     * @param params User Operation Params
     * @returns Gas Estimate
     */
    estimateGas(params: UserOperationParams): Promise<GasEstimate>;
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

interface RampOSClientOptions {
    baseURL?: string;
    apiKey: string;
    timeout?: number;
}
declare class RampOSClient {
    private readonly httpClient;
    readonly intents: IntentService;
    readonly users: UserService;
    readonly ledger: LedgerService;
    readonly aa: AAService;
    readonly webhooks: WebhookVerifier;
    constructor(options: RampOSClientOptions);
}

export { AAService, type AddSessionKeyParams, AddSessionKeyParamsSchema, type CreateAccountParams, CreateAccountParamsSchema, type CreatePayInDto, CreatePayInSchema, type CreatePayOutDto, CreatePayOutSchema, type GasEstimate, GasEstimateSchema, type Intent, IntentFilterSchema, type IntentFilters, IntentSchema, IntentService, IntentStatus, IntentType, KycStatus, type LedgerEntry, LedgerEntrySchema, LedgerEntryType, LedgerFilterSchema, type LedgerFilters, LedgerService, RampOSClient, type RampOSClientOptions, type RemoveSessionKeyParams, RemoveSessionKeyParamsSchema, type SessionKey, SessionKeySchema, type SmartAccount, SmartAccountSchema, type UserBalance, UserBalanceSchema, type UserKycStatus, UserKycStatusSchema, type UserOpReceipt, UserOpReceiptSchema, type UserOperation, type UserOperationParams, UserOperationParamsSchema, UserOperationSchema, UserService, WebhookVerifier };
