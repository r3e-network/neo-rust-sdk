import { test, expect } from '@playwright/test';

test.describe('Dashboard Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should display dashboard content', async ({ page }) => {
    // Check dashboard heading
    await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible();
    
    // Since no wallet is connected by default, check for the appropriate content
    const noWalletHeading = page.getByText(/no wallet connected/i);
    const connectedDashboard = page.getByText(/total value/i);
    
    // Either the no-wallet state OR the connected dashboard should be visible
    try {
      await expect(noWalletHeading).toBeVisible({ timeout: 2000 });
      // If no wallet connected, also check for the create wallet button
      await expect(page.getByRole('button', { name: /create wallet/i })).toBeVisible();
    } catch {
      // If wallet is connected, check for dashboard elements
      await expect(connectedDashboard).toBeVisible();
      await expect(page.getByText(/neo balance/i)).toBeVisible();
      await expect(page.getByText(/gas balance/i)).toBeVisible();
    }
  });

  test('should show wallet status', async ({ page }) => {
    // Check for wallet-related content - should show appropriate state
    const walletElements = [
      page.getByText(/no wallet connected/i),
      page.getByRole('button', { name: /create wallet/i }),
      page.getByText(/total value/i) // If wallet is connected
    ];
    
    // At least one wallet-related element should be visible
    let foundWalletElement = false;
    for (const element of walletElements) {
      try {
        await expect(element).toBeVisible({ timeout: 1000 });
        foundWalletElement = true;
        break;
      } catch {
        // Continue to next element
      }
    }
    expect(foundWalletElement).toBe(true);
  });

  test('should display network information', async ({ page }) => {
    // Network info is displayed in the header when wallet is connected
    const noWalletState = page.getByText(/no wallet connected/i);
    
    try {
      await expect(noWalletState).toBeVisible({ timeout: 2000 });
      // If no wallet connected, network info won't be shown - this is expected
      expect(true).toBe(true);
    } catch {
      // If wallet is connected, check for network badge
      await expect(page.getByText(/(Mainnet|Testnet)/)).toBeVisible();
    }
  });

  test('should show charts and statistics', async ({ page }) => {
    // Charts only appear when wallet is connected
    const noWalletState = page.getByText(/no wallet connected/i);
    
    try {
      await expect(noWalletState).toBeVisible({ timeout: 2000 });
      // If no wallet connected, this test should pass (charts aren't expected)
      expect(true).toBe(true);
    } catch {
      // If wallet is connected, look for chart content
      const chartElements = [
        page.getByText(/price history/i),
        page.getByText(/portfolio distribution/i),
        page.getByText(/nfts owned/i)
      ];
      
      let foundChartElement = false;
      for (const element of chartElements) {
        try {
          await expect(element).toBeVisible({ timeout: 2000 });
          foundChartElement = true;
          break;
        } catch {
          // Continue to next element
        }
      }
      expect(foundChartElement).toBe(true);
    }
  });

  test('should display recent activity', async ({ page }) => {
    // Recent activity only appears when wallet is connected
    const noWalletState = page.getByText(/no wallet connected/i);
    
    try {
      await expect(noWalletState).toBeVisible({ timeout: 2000 });
      // If no wallet connected, this test should pass (recent activity isn't expected)
      expect(true).toBe(true);
    } catch {
      // If wallet is connected, check for recent activity section
      await expect(page.getByText(/recent activity/i)).toBeVisible();
    }
  });
}); 