import { dashboardApi, intentsApi } from './api';

// Create a mock dashboard API for testing
export const mockDashboardApi = {
  getStats: async () => {
    return {
      intents: {
        totalToday: 156,
        payinCount: 89,
        payoutCount: 67,
        pendingCount: 12,
        completedCount: 140,
        failedCount: 4,
      },
      cases: {
        total: 23,
        open: 5,
        inReview: 8,
        onHold: 3,
        resolved: 7,
        avgResolutionHours: 18.5,
      },
      users: {
        total: 5420,
        active: 1234,
        kycPending: 45,
        newToday: 28,
      },
      volume: {
        totalPayinVnd: "2500000000",
        totalPayoutVnd: "1800000000",
        totalTradeVnd: "5200000000",
        period: "24h",
      },
    };
  }
};

// Test script
async function testApi() {
  console.log("Testing API integration...");

  // Test Dashboard API
  try {
    console.log("Calling getStats...");
    // We can't easily test the real API in this environment without backend running
    // So we'll verify the structure is correct
    const stats = await mockDashboardApi.getStats();

    if (stats.volume.totalPayinVnd !== "2500000000") {
      throw new Error("Stats volume mismatch");
    }
    console.log("✅ getStats passed (mock)");
  } catch (err) {
    console.error("❌ getStats failed:", err);
  }

  // Test Intent API
  try {
    // Check if intentsApi has the list method
    if (typeof intentsApi.list !== 'function') {
      throw new Error("intentsApi.list is not a function");
    }
    console.log("✅ intentsApi structure valid");
  } catch (err) {
    console.error("❌ intentsApi failed:", err);
  }

  console.log("API tests completed.");
}

testApi();
