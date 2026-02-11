import { gql } from 'urql';

// ============================================================================
// Queries
// ============================================================================

export const GET_INTENT = gql`
  query GetIntent($tenantId: String!, $id: ID!) {
    intent(tenantId: $tenantId, id: $id) {
      id
      tenantId
      userId
      intentType
      state
      stateHistory
      amount
      currency
      actualAmount
      railsProvider
      referenceCode
      bankTxId
      chainId
      txHash
      fromAddress
      toAddress
      metadata
      idempotencyKey
      createdAt
      updatedAt
      expiresAt
      completedAt
    }
  }
`;

export const GET_INTENTS = gql`
  query GetIntents(
    $tenantId: String!
    $filter: IntentFilter
    $first: Int
    $after: String
  ) {
    intents(tenantId: $tenantId, filter: $filter, first: $first, after: $after) {
      edges {
        cursor
        node {
          id
          tenantId
          userId
          intentType
          state
          amount
          currency
          referenceCode
          createdAt
          updatedAt
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_USER = gql`
  query GetUser($tenantId: String!, $id: ID!) {
    user(tenantId: $tenantId, id: $id) {
      id
      tenantId
      kycTier
      kycStatus
      kycVerifiedAt
      riskScore
      riskFlags
      dailyPayinLimitVnd
      dailyPayoutLimitVnd
      status
      createdAt
      updatedAt
    }
  }
`;

export const GET_USERS = gql`
  query GetUsers($tenantId: String!, $first: Int, $after: String) {
    users(tenantId: $tenantId, first: $first, after: $after) {
      edges {
        cursor
        node {
          id
          tenantId
          kycTier
          kycStatus
          status
          createdAt
          updatedAt
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_DASHBOARD_STATS = gql`
  query GetDashboardStats($tenantId: String!) {
    dashboardStats(tenantId: $tenantId) {
      totalUsers
      activeUsers
      totalIntentsToday
      totalPayinVolumeToday
      totalPayoutVolumeToday
      pendingIntents
    }
  }
`;

// ============================================================================
// Mutations
// ============================================================================

export const CREATE_PAY_IN = gql`
  mutation CreatePayIn($tenantId: String!, $input: CreatePayInInput!) {
    createPayIn(tenantId: $tenantId, input: $input) {
      intentId
      referenceCode
      status
      expiresAt
      dailyLimit
      dailyRemaining
    }
  }
`;

export const CONFIRM_PAY_IN = gql`
  mutation ConfirmPayIn($tenantId: String!, $input: ConfirmPayInInput!) {
    confirmPayIn(tenantId: $tenantId, input: $input) {
      intentId
      success
    }
  }
`;

export const CREATE_PAYOUT = gql`
  mutation CreatePayout($tenantId: String!, $input: CreatePayoutInput!) {
    createPayout(tenantId: $tenantId, input: $input) {
      intentId
      status
      dailyLimit
      dailyRemaining
    }
  }
`;

// ============================================================================
// Subscriptions
// ============================================================================

export const INTENT_STATUS_CHANGED = gql`
  subscription IntentStatusChanged($tenantId: String!) {
    intentStatusChanged(tenantId: $tenantId) {
      intentId
      tenantId
      newStatus
      timestamp
    }
  }
`;
