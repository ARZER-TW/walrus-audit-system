/**
 * useSuiContract Hook
 *
 * Used to interact with backend Sui contract client
 * Provides on-chain contract query functionality
 */

import { useState, useEffect } from 'react';

const SEAL_API_URL = import.meta.env.VITE_SEAL_API_URL || 'http://localhost:3001';

/**
 * Sui Contract Test Result
 */
export interface SuiContractTestResult {
  success: boolean;
  summary: {
    total: number;
    passed: number;
    failed: number;
  };
  results: {
    timestamp: string;
    tests: Array<{
      name: string;
      status: 'success' | 'failed';
      data?: any;
      error?: string;
    }>;
  };
}

/**
 * AuditConfig Data Structure
 */
export interface AuditConfig {
  admin: string;
  auditor_stakes: any;
  authorized_auditors: string[];
  challenge_interval_ms: string;
  id: { id: string };
  max_challenge_count: number;
  min_challenge_count: number;
  total_audits: string;
  total_blobs_audited: string;
}

/**
 * useSuiContract Hook
 *
 * Provides Sui contract query functionality
 */
export function useSuiContract() {
  const [isConnected, setIsConnected] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<SuiContractTestResult | null>(null);

  /**
   * Test connection to backend Sui contract client
   */
  const testConnection = async (): Promise<SuiContractTestResult | null> => {
    try {
      setIsLoading(true);
      setError(null);

      const response = await fetch(`${SEAL_API_URL}/api/sui/test`);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data: SuiContractTestResult = await response.json();

      setTestResults(data);
      setIsConnected(data.success && data.summary.passed === data.summary.total);

      return data;
    } catch (err: any) {
      const errorMsg = err.message || 'Failed to connect to Sui contract client';
      setError(errorMsg);
      setIsConnected(false);
      console.error('❌ Sui contract test failed:', err);
      return null;
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * Extract AuditConfig data from test results
   */
  const getAuditConfigFromTest = (): AuditConfig | null => {
    if (!testResults || !testResults.results) {
      return null;
    }

    const auditConfigTest = testResults.results.tests.find(
      test => test.name === '讀取 AuditConfig' && test.status === 'success'
    );

    return auditConfigTest?.data || null;
  };

  /**
   * Check if auditor is registered
   */
  const isAuditorRegistered = (): boolean | null => {
    if (!testResults || !testResults.results) {
      return null;
    }

    const registrationTest = testResults.results.tests.find(
      test => test.name === '檢查審計員註冊' && test.status === 'success'
    );

    return registrationTest?.data?.isRegistered ?? null;
  };

  /**
   * Get auditor reputation score
   */
  const getAuditorReputation = (): number | null => {
    if (!testResults || !testResults.results) {
      return null;
    }

    const reputationTest = testResults.results.tests.find(
      test => test.name === '查詢聲譽分數' && test.status === 'success'
    );

    return reputationTest?.data?.reputation ?? null;
  };

  /**
   * Automatically test connection on component mount
   */
  useEffect(() => {
    testConnection();
  }, []);

  return {
    // State
    isConnected,
    isLoading,
    error,
    testResults,

    // Methods
    testConnection,
    getAuditConfigFromTest,
    isAuditorRegistered,
    getAuditorReputation,
  };
}
