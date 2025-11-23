/**
 * useSuiContract Hook
 *
 * 用於與後端 Sui 合約客戶端交互
 * 提供鏈上合約查詢功能
 */

import { useState, useEffect } from 'react';

const SEAL_API_URL = import.meta.env.VITE_SEAL_API_URL || 'http://localhost:3001';

/**
 * Sui 合約測試結果
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
 * AuditConfig 數據結構
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
 * 提供 Sui 合約查詢功能
 */
export function useSuiContract() {
  const [isConnected, setIsConnected] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<SuiContractTestResult | null>(null);

  /**
   * 測試與後端 Sui 合約客戶端的連接
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
      const errorMsg = err.message || '連接 Sui 合約客戶端失敗';
      setError(errorMsg);
      setIsConnected(false);
      console.error('❌ Sui 合約測試失敗:', err);
      return null;
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * 從測試結果中提取 AuditConfig 數據
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
   * 檢查審計員是否已註冊
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
   * 獲取審計員聲譽分數
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
   * 組件掛載時自動測試連接
   */
  useEffect(() => {
    testConnection();
  }, []);

  return {
    // 狀態
    isConnected,
    isLoading,
    error,
    testResults,

    // 方法
    testConnection,
    getAuditConfigFromTest,
    isAuditorRegistered,
    getAuditorReputation,
  };
}
