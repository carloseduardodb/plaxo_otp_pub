import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Copy, Trash2, Check, Clock, Edit3, X } from 'lucide-react';
import { getPlatformIcon, getPlatformColor } from '../utils/platformIcons';
import { useOtpTimer } from '../hooks/useOtpTimer';

interface Props {
  app: {
    id: string;
    name: string;
    secret: string;
  };
  onDelete: (id: string) => void;
  onEdit?: () => void;
  isVisible?: boolean;
}

export default function OtpItem({ app, onDelete, onEdit, isVisible = true }: Props) {
  const [otp, setOtp] = useState('------');
  const [copied, setCopied] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(app.name);
  const [isGenerating, setIsGenerating] = useState(false);

  const { timeLeft, shouldRefresh } = useOtpTimer();

  useEffect(() => {
    if (!isVisible) {
      setCopied(false);
      setIsEditing(false);
    }
  }, [isVisible]);

  const generateOtp = useCallback(async () => {
    if (isGenerating || !isVisible) return;

    setIsGenerating(true);
    try {
      const code = await invoke<string>('generate_otp', { appId: app.id });
      setOtp(code);
    } catch (error) {
      console.error('Failed to generate OTP:', error);
      setOtp('ERROR');
    } finally {
      setIsGenerating(false);
    }
  }, [app.id, isGenerating, isVisible]);

  useEffect(() => {
    if (isVisible) {
      generateOtp();
    }
  }, [isVisible]);

  useEffect(() => {
    if (shouldRefresh && isVisible) {
      generateOtp();
    }
  }, [shouldRefresh, generateOtp, isVisible]);

  const handleEdit = async () => {
    if (editName.trim() && editName !== app.name) {
      try {
        await invoke('edit_app_name', { id: app.id, newName: editName.trim() });
        onEdit?.();
      } catch (error) {
        console.error('Failed to edit app name:', error);
      }
    }
    setIsEditing(false);
  };

  const handleCancelEdit = () => {
    setEditName(app.name);
    setIsEditing(false);
  };

  const copyToClipboard = async () => {
    if (otp && otp !== 'ERROR') {
      try {
        await invoke('copy_to_clipboard', { text: otp });
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch (error) {
        console.error('Failed to copy to clipboard:', error);
      }
    }
  };

  const progressPercentage = (timeLeft / 30) * 100;
  const isExpiring = timeLeft <= 10;

  const PlatformIcon = getPlatformIcon(app.name);
  const platformColor = getPlatformColor(app.name);

  return (
    <div className="bg-plaxo-background/30 border border-plaxo-border rounded-xl p-4 space-y-4 hover:bg-plaxo-background/50 transition-colors">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div
            className="flex items-center justify-center w-10 h-10 rounded-xl"
            style={{ backgroundColor: `${platformColor}15`, color: platformColor }}
          >
            <PlatformIcon className="w-5 h-5" />
          </div>
          {isEditing ? (
            <div className="flex items-center gap-2 flex-1">
              <input
                type="text"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                className="flex-1 bg-transparent text-plaxo-text font-semibold border-b border-plaxo-primary focus:outline-none"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleEdit();
                  if (e.key === 'Escape') handleCancelEdit();
                }}
                autoFocus
              />
              <button
                onClick={handleEdit}
                className="text-green-500 hover:text-green-600 p-1"
              >
                <Check className="w-4 h-4" />
              </button>
              <button
                onClick={handleCancelEdit}
                className="text-red-500 hover:text-red-600 p-1"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          ) : (
            <h3 className="font-semibold text-plaxo-text truncate">{app.name}</h3>
          )}
        </div>
        <div className="flex items-center gap-1">
          {!isEditing && (
            <button
              onClick={() => setIsEditing(true)}
              className="text-plaxo-text-secondary hover:text-plaxo-text p-1.5 rounded-lg hover:bg-plaxo-hover transition-colors"
              title="Editar nome"
            >
              <Edit3 className="w-4 h-4" />
            </button>
          )}
          <button
            onClick={() => onDelete(app.id)}
            className="text-plaxo-text-secondary hover:text-plaxo-error p-1.5 rounded-lg hover:bg-plaxo-error/10 transition-colors"
            title="Remover aplicativo"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      </div>

      <div className="space-y-3">
        <div className="text-center">
          <div className={`font-mono text-3xl font-bold mb-2 tracking-wider select-all ${otp === 'ERROR' ? 'text-red-500' : 'text-plaxo-primary'
            }`}>
            {otp === 'ERROR' ? 'INVALID' : (otp || '------')}
          </div>
          {otp === 'ERROR' && (
            <div className="text-xs text-red-500 mb-2">
              Chave secreta inválida
            </div>
          )}
        </div>

        <div className="space-y-2">
          <div className="otp-progress">
            <div
              className={`otp-progress-bar ${isExpiring ? 'bg-plaxo-warning' : 'bg-plaxo-primary'}`}
              style={{ width: `${progressPercentage}%` }}
            />
          </div>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-xs text-plaxo-text-secondary">
              <Clock className="w-3 h-3" />
              <span>{timeLeft}s restantes</span>
            </div>
            <button
              onClick={copyToClipboard}
              disabled={otp === 'ERROR'}
              className="flex items-center gap-2 px-3 py-1.5 bg-plaxo-primary hover:bg-plaxo-primary-hover text-plaxo-background text-sm font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {copied ? (
                <>
                  <Check className="w-3 h-3" />
                  Copiado!
                </>
              ) : (
                <>
                  <Copy className="w-3 h-3" />
                  Copiar
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
