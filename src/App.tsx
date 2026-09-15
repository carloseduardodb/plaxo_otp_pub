import { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { Shield, Lock } from 'lucide-react';
import MasterPasswordModal from './components/MasterPasswordModal';
import VirtualizedOtpList from './components/VirtualizedOtpList';
import SearchBar from './components/SearchBar';
import AddAppModal from './components/AddAppModal';
import ImportModal from './components/ImportModal';
import ConfirmModal from './components/ConfirmModal';
import { useDebounce } from './hooks/useDebounce';
import { useMemoryManager } from './hooks/useMemoryManager';

import { GoogleDriveSync } from './components/GoogleDriveSync';

interface OtpApp {
  id: string;
  name: string;
  secret: string;
}

function App() {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [apps, setApps] = useState<OtpApp[]>([]);
  const [searchTerm, setSearchTerm] = useState('');
  const [showAddModal, setShowAddModal] = useState(false);
  const [showImportModal, setShowImportModal] = useState(false);
  const [showResetModal, setShowResetModal] = useState(false);
  const [hasMasterPassword, setHasMasterPassword] = useState(false);
  const [isVisible, setIsVisible] = useState(true);
  const cleanupRef = useRef<NodeJS.Timeout>();
  const { forceCleanup } = useMemoryManager();

  useEffect(() => {
    const handleVisibilityChange = () => {
      const visible = !document.hidden;
      setIsVisible(visible);

      if (!visible) {
        cleanupRef.current = setTimeout(() => {
          setSearchTerm('');
          forceCleanup();
        }, 5000);
      } else {
        if (cleanupRef.current) {
          clearTimeout(cleanupRef.current);
        }
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      if (cleanupRef.current) {
        clearTimeout(cleanupRef.current);
      }
    };
  }, [forceCleanup]);

  useEffect(() => {
    const checkMasterPassword = async () => {
      try {
        const exists = await invoke<boolean>('has_master_password');
        setHasMasterPassword(exists);
      } catch (error) {
        console.error('Failed to check master password:', error);
      }
    };
    checkMasterPassword();
  }, []);

  // The backend locks the vault after a period without real input and emits
  // this event. Drop the decrypted codes held in React state and return to the
  // password prompt, otherwise the UI would keep showing them.
  useEffect(() => {
    const unlisten = listen('vault-locked', () => {
      setIsAuthenticated(false);
      setApps([]);
      setSearchTerm('');
      setShowAddModal(false);
      setShowImportModal(false);
    });

    return () => {
      void unlisten.then((off) => off());
    };
  }, []);

  // Report genuine interaction so the auto-lock only fires when the user is
  // actually away. Throttled, since these events fire constantly.
  useEffect(() => {
    if (!isAuthenticated) return;

    let lastSent = 0;
    const report = () => {
      const now = Date.now();
      if (now - lastSent < 20_000) return;
      lastSent = now;
      void invoke('touch_activity').catch(() => {
        // A failed heartbeat should never break the UI; the worst case is an
        // earlier lock.
      });
    };

    report();

    const events: Array<keyof DocumentEventMap> = [
      'mousemove',
      'mousedown',
      'keydown',
      'wheel',
      'touchstart',
    ];
    events.forEach((name) => document.addEventListener(name, report, { passive: true }));

    return () => {
      events.forEach((name) => document.removeEventListener(name, report));
    };
  }, [isAuthenticated]);

  const loadApps = useCallback(async () => {
    try {
      const loadedApps = await invoke<OtpApp[]>('get_apps');
      setApps(loadedApps);
    } catch (error) {
      console.error('Failed to load apps:', error);
    }
  }, []);

  const handleMasterPassword = useCallback(async (password: string) => {
    try {
      const isValid = await invoke<boolean>('verify_master_password', { password });
      if (isValid) {
        setIsAuthenticated(true);
        await loadApps();
      }
      return isValid;
    } catch (error) {
      console.error('Failed to verify password:', error);
      return false;
    }
  }, [loadApps]);

  const handleReset = useCallback(async () => {
    try {
      await invoke('reset_master_password');
      setIsAuthenticated(false);
      setApps([]);
      setHasMasterPassword(false);
      setShowResetModal(false);
    } catch (error) {
      console.error('Failed to reset:', error);
    }
  }, []);

  const handleAddApp = useCallback(async (name: string, secret: string) => {
    try {
      await invoke('add_app', { name, secret });
      await loadApps();
    } catch (error) {
      console.error('Failed to add app:', error);
      // Re-throw with better error message
      const errorMsg = typeof error === 'string' ? error : 'Erro ao adicionar aplicativo';
      throw new Error(errorMsg);
    }
  }, [loadApps]);

  const handleDeleteApp = useCallback(async (id: string) => {
    try {
      await invoke('delete_app', { appId: id });
      await loadApps();
    } catch (error) {
      console.error('Failed to delete app:', error);
    }
  }, [loadApps]);

  const debouncedSearchTerm = useDebounce(searchTerm, 300);

  const filteredApps = useMemo(() => {
    if (!debouncedSearchTerm.trim()) {
      return apps;
    }
    return apps.filter(app =>
      app.name.toLowerCase().includes(debouncedSearchTerm.toLowerCase())
    );
  }, [apps, debouncedSearchTerm]);

  const displayApps = useMemo(() => {
    if (!isVisible && filteredApps.length > 10) {
      return filteredApps.slice(0, 10);
    }
    return filteredApps;
  }, [filteredApps, isVisible]);

  if (!isAuthenticated) {
    return (
      <div className="auth-container">
        <MasterPasswordModal
          onSubmit={handleMasterPassword}
          onReset={() => setShowResetModal(true)}
          isFirstTime={!hasMasterPassword}
        />

        <ConfirmModal
          isOpen={showResetModal}
          title="Resetar Senha Mestre"
          message="Esta ação irá apagar TODOS os seus dados e aplicativos cadastrados. Você terá que configurar uma nova senha mestre e adicionar todos os aplicativos novamente. Esta ação não pode ser desfeita."
          confirmText="Sim, resetar tudo"
          cancelText="Cancelar"
          variant="danger"
          onConfirm={handleReset}
          onCancel={() => setShowResetModal(false)}
        />
      </div>
    );
  }

  return (
    <div className="main-container relative">
      <div className="content-container">
        <div className="header-sticky">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-3">
              <div className="flex items-center justify-center w-10 h-10 bg-plaxo-primary/10 rounded-xl">
                <Shield className="w-5 h-5 text-plaxo-primary" />
              </div>
              <h1 className="text-xl font-heading font-bold text-plaxo-text">
                Plaxo OTP
              </h1>
            </div>
            <GoogleDriveSync />
          </div>
          <SearchBar
            value={searchTerm}
            onChange={setSearchTerm}
            onAddClick={() => setShowAddModal(true)}
            onImportClick={() => setShowImportModal(true)}
          />
        </div>
        <div className="content-scrollable">
          {displayApps.length === 0 ? (
            <div className="flex items-center justify-center h-full">
              <div className="text-center text-plaxo-text-secondary">
                <div className="inline-flex items-center justify-center w-16 h-16 bg-plaxo-border/20 rounded-2xl mb-4">
                  <Lock className="w-8 h-8" />
                </div>
                <p className="text-lg font-medium mb-2">Nenhum aplicativo cadastrado</p>
                <p className="text-sm">Adicione seu primeiro aplicativo para começar a gerar códigos OTP</p>
              </div>
            </div>
          ) : (
            <VirtualizedOtpList
              apps={displayApps}
              onDelete={handleDeleteApp}
              onEdit={loadApps}
              isVisible={isVisible}
            />
          )}
        </div>

        {showAddModal && (
          <AddAppModal
            onSubmit={handleAddApp}
            onClose={() => setShowAddModal(false)}
          />
        )}

        {showImportModal && (
          <ImportModal
            onClose={() => setShowImportModal(false)}
            onImportComplete={loadApps}
          />
        )}
      </div>
    </div>
  );
}

export default App;
