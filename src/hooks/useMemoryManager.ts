import { useEffect, useRef } from "react";

interface MemoryStats {
  usedJSHeapSize?: number;
  totalJSHeapSize?: number;
  jsHeapSizeLimit?: number;
}

declare global {
  interface Performance {
    memory?: MemoryStats;
  }
}

export function useMemoryManager() {
  const cleanupIntervalRef = useRef<NodeJS.Timeout>();
  const lastCleanupRef = useRef<number>(0);

  useEffect(() => {
    const performCleanup = () => {
      const now = Date.now();

      if (now - lastCleanupRef.current < 30000) return;

      lastCleanupRef.current = now;

      const memInfo = performance.memory;
      if (memInfo) {
        const usedMB = (memInfo.usedJSHeapSize || 0) / 1024 / 1024;

        if (usedMB > 100) {
          console.log(`Memory usage: ${usedMB.toFixed(1)}MB - cleaning up...`);

          document
            .querySelectorAll('[data-cleanup="true"], .unused')
            .forEach((el) => {
              el.remove();
            });

          const reactRoot = document.querySelector("#root") as HTMLElement;
          if (reactRoot) {
            reactRoot.style.display = "none";
            setTimeout(() => {
              reactRoot.style.display = "";
            }, 1);
          }

          if (window.gc) {
            window.gc();
          }

          console.clear();
        }
      }
    };

    const handleVisibilityChange = () => {
      if (document.hidden) {
        cleanupIntervalRef.current = setInterval(performCleanup, 10000);
      } else {
        if (cleanupIntervalRef.current) {
          clearInterval(cleanupIntervalRef.current);
          cleanupIntervalRef.current = undefined;
        }
      }
    };

    handleVisibilityChange();
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      if (cleanupIntervalRef.current) {
        clearInterval(cleanupIntervalRef.current);
      }
    };
  }, []);

  const forceCleanup = () => {
    if (window.gc) {
      window.gc();
    }

    if (window.otpCache) {
      window.otpCache = {};
    }

    console.log("Manual memory cleanup performed");
  };

  return { forceCleanup };
}
