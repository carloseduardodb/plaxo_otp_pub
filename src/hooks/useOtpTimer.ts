import { useState, useEffect, useRef } from "react";

interface UseOtpTimerReturn {
  timeLeft: number;
  shouldRefresh: boolean;
}

export function useOtpTimer(): UseOtpTimerReturn {
  const [timeLeft, setTimeLeft] = useState(30);
  const [shouldRefresh, setShouldRefresh] = useState(false);
  const [isVisible, setIsVisible] = useState(true);
  const intervalRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    const handleVisibilityChange = () => {
      setIsVisible(!document.hidden);
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, []);

  useEffect(() => {
    const updateTimer = () => {
      if (!isVisible) return;
      
      const now = Math.floor(Date.now() / 1000);
      const remaining = 30 - (now % 30);
      setTimeLeft(remaining);

      if (remaining === 30) {
        setShouldRefresh(true);
        setTimeout(() => setShouldRefresh(false), 100);
      }
    };

    if (isVisible) {
      updateTimer();
      intervalRef.current = setInterval(updateTimer, 1000);
    } else {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [isVisible]);

  return { timeLeft, shouldRefresh };
}
