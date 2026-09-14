export const PERFORMANCE_CONFIG = {
  SEARCH_DEBOUNCE_DELAY: 300,
  MAX_CONCURRENT_OTP_GENERATIONS: 5,
  TIMER_UPDATE_INTERVAL: 1000,
  MEMORY_CLEANUP_INTERVAL: 300000, // 5 minutes instead of 1
  MAX_VISIBLE_APPS: 50,
} as const;

export class MemoryManager {
  private static instance: MemoryManager;
  private cleanupInterval: NodeJS.Timeout | null = null;
  private isVisible = true;

  private constructor() {
    document.addEventListener("visibilitychange", () => {
      this.isVisible = !document.hidden;
    });
  }

  static getInstance(): MemoryManager {
    if (!MemoryManager.instance) {
      MemoryManager.instance = new MemoryManager();
    }
    return MemoryManager.instance;
  }

  startCleanup(): void {
    if (this.cleanupInterval) return;

    this.cleanupInterval = setInterval(() => {
      if (!this.isVisible) {
        this.clearStaleCache();
        if (window.gc && import.meta.env.PROD) {
          window.gc();
        }
      }
    }, PERFORMANCE_CONFIG.MEMORY_CLEANUP_INTERVAL);
  }

  stopCleanup(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
  }

  private clearStaleCache(): void {
    if (window.otpCache && typeof window.otpCache === "object") {
      const now = Date.now();
      Object.keys(window.otpCache).forEach((key) => {
        if (window.otpCache && now - window.otpCache[key].timestamp > 30000) {
          delete window.otpCache[key];
        }
      });
    }

    const unusedElements = document.querySelectorAll('[data-cleanup="true"]');
    unusedElements.forEach((el) => el.remove());

    if (import.meta.env.PROD && !this.isVisible) {
      console.clear();
    }
  }
}

declare global {
  interface Window {
    gc?: () => void;
    otpCache?: Record<string, { code: string; timestamp: number }>;
    React?: any;
  }
}
