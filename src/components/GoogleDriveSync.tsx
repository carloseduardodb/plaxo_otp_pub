import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Cloud, CloudOff } from 'lucide-react';

export const GoogleDriveSync: React.FC = () => {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isConfigured, setIsConfigured] = useState(true);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void init();
  }, []);

  const init = async () => {
    // Builds without Google credentials have no sync to offer; say so instead
    // of showing a button that can only fail.
    try {
      const configured = await invoke<boolean>('google_drive_is_configured');
      setIsConfigured(configured);
      if (!configured) return;
    } catch {
      setIsConfigured(false);
      return;
    }

    await checkAuth();
  };

  const checkAuth = async () => {
    try {
      const authenticated = await invoke<boolean>('check_google_auth');
      setIsAuthenticated(authenticated);
    } catch {
      setIsAuthenticated(false);
    }
  };

  const handleAuth = async () => {
    try {
      setIsLoading(true);
      setError(null);
      await invoke('google_drive_auth_flow');
      setIsAuthenticated(true);
    } catch (err) {
      // Surface the reason: the flow can fail on a denied consent, a timeout,
      // or a rejected callback, and silence makes those indistinguishable.
      setError(typeof err === 'string' ? err : 'Falha ao conectar ao Google Drive');
      console.error('Erro na autenticação:', err);
    } finally {
      setIsLoading(false);
    }
  };

  if (!isConfigured) {
    return (
      <div className="flex items-center">
        <div
          className="flex items-center justify-center w-10 h-10 bg-plaxo-surface border border-plaxo-border rounded-xl opacity-40"
          title="Sync com Google Drive não configurado nesta build (veja docs/google-drive-setup.md)"
        >
          <CloudOff size={18} />
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center">
      {!isAuthenticated ? (
        <button
          onClick={handleAuth}
          disabled={isLoading}
          className="flex items-center justify-center w-10 h-10 bg-plaxo-surface hover:bg-plaxo-surface-hover text-plaxo-text-secondary hover:text-plaxo-text border border-plaxo-border rounded-xl transition-colors disabled:opacity-50"
          title={error ?? 'Conectar Google Drive (Sincronização Automática)'}
        >
          <CloudOff size={18} className={error ? 'text-red-400' : undefined} />
        </button>
      ) : (
        <div
          className="flex items-center justify-center w-10 h-10 bg-plaxo-surface border border-plaxo-border rounded-xl"
          title="Google Drive conectado - Sincronização automática ativa"
        >
          <Cloud size={18} className="text-plaxo-primary" />
        </div>
      )}
    </div>
  );
};
