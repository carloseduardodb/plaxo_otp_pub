import { useState } from 'react';
import { Plus, X, Loader2, AlertCircle, Image } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';

interface Props {
  onSubmit: (name: string, secret: string) => Promise<void>;
  onClose: () => void;
}

export default function AddAppModal({ onSubmit, onClose }: Props) {
  const [name, setName] = useState('');
  const [secret, setSecret] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [processingQR, setProcessingQR] = useState(false);

  const handlePaste = async (e: React.ClipboardEvent) => {
    e.preventDefault();
    setProcessingQR(true);
    setError('');
    
    try {
      const qrData = await invoke<{ name: string; secret: string }>('decode_qr_from_clipboard');
      
      if (!qrData.secret || qrData.secret.trim().length === 0) {
        throw new Error('QR code não contém dados válidos');
      }
      
      setSecret(qrData.secret.trim());
      if (qrData.name && qrData.name !== 'Unknown') {
        setName(qrData.name);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      // Se não for imagem, permite paste normal
      if (!errorMsg.includes('Nenhuma imagem')) {
        setError(`Erro ao processar QR: ${errorMsg}`);
      }
    } finally {
      setProcessingQR(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    const trimmedName = name.trim();
    const trimmedSecret = secret.trim();
    
    if (!trimmedName) {
      setError('Nome do aplicativo é obrigatório');
      return;
    }
    
    if (!trimmedSecret) {
      setError('Chave secreta é obrigatória');
      return;
    }

    setLoading(true);
    setError('');

    try {
      await onSubmit(trimmedName, trimmedSecret);
      onClose();
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg || 'Erro ao adicionar aplicativo');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 z-50">
      <div className="bg-plaxo-surface border border-plaxo-border rounded-2xl p-6 w-full max-w-md shadow-2xl">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div className="flex items-center justify-center w-10 h-10 bg-plaxo-primary/10 rounded-xl">
              <Plus className="w-5 h-5 text-plaxo-primary" />
            </div>
            <h2 className="text-lg font-heading font-semibold text-plaxo-text">
              Adicionar Aplicativo
            </h2>
          </div>
          <button
            onClick={onClose}
            className="text-plaxo-text-secondary hover:text-plaxo-text p-1.5 rounded-lg hover:bg-plaxo-background/50 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-5">
          <div className="space-y-2">
            <label className="text-sm font-medium text-plaxo-text">
              Nome do Aplicativo
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Ex: Google, GitHub, Discord..."
              className="w-full px-4 py-3 bg-plaxo-background/50 border border-plaxo-border rounded-xl text-plaxo-text placeholder-plaxo-text-secondary focus:outline-none focus:border-plaxo-primary focus:ring-2 focus:ring-plaxo-primary/20 transition-all"
              disabled={loading}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium text-plaxo-text">
              Chave Secreta
            </label>
            <div className="relative">
              <input
                type="text"
                value={secret}
                onChange={(e) => setSecret(e.target.value)}
                onPaste={handlePaste}
                placeholder="Cole aqui o código secreto ou Ctrl+V uma imagem QR..."
                className="w-full px-4 py-3 bg-plaxo-background/50 border border-plaxo-border rounded-xl text-plaxo-text placeholder-plaxo-text-secondary focus:outline-none focus:border-plaxo-primary focus:ring-2 focus:ring-plaxo-primary/20 transition-all font-mono text-sm"
                disabled={loading || processingQR}
              />
              {processingQR && (
                <div className="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-2 text-plaxo-primary">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  <span className="text-xs">Lendo QR...</span>
                </div>
              )}
            </div>
            <p className="text-xs text-plaxo-text-secondary flex items-center gap-1">
              <Image className="w-3 h-3" />
              Dica: Cole uma imagem QR code com Ctrl+V para extrair automaticamente
            </p>
          </div>

          {error && (
            <div className="flex items-center gap-2 text-plaxo-error text-sm bg-plaxo-error/10 px-3 py-2 rounded-lg border border-plaxo-error/20">
              <AlertCircle className="w-4 h-4" />
              {error}
            </div>
          )}

          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 py-3 px-4 bg-plaxo-background/50 hover:bg-plaxo-background/70 text-plaxo-text border border-plaxo-border rounded-xl transition-colors font-medium"
              disabled={loading}
            >
              Cancelar
            </button>
            <button
              type="submit"
              disabled={loading || !name.trim() || !secret.trim()}
              className="flex-1 flex items-center justify-center gap-2 py-3 px-4 bg-plaxo-primary hover:bg-plaxo-primary-hover text-plaxo-background font-semibold rounded-xl transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Salvando...
                </>
              ) : (
                <>
                  <Plus className="w-4 h-4" />
                  Adicionar
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
