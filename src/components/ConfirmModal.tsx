import { AlertTriangle, X } from 'lucide-react';

interface Props {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  onConfirm: () => void;
  onCancel: () => void;
  variant?: 'danger' | 'warning';
}

export default function ConfirmModal({
  isOpen,
  title,
  message,
  confirmText = 'Confirmar',
  cancelText = 'Cancelar',
  onConfirm,
  onCancel,
  variant = 'warning'
}: Props) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-plaxo-surface border border-plaxo-border rounded-lg p-6 max-w-md w-full mx-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <AlertTriangle 
              className={`w-6 h-6 ${variant === 'danger' ? 'text-red-500' : 'text-yellow-500'}`} 
            />
            <h3 className="text-lg font-semibold text-plaxo-text-primary">
              {title}
            </h3>
          </div>
          <button
            onClick={onCancel}
            className="text-plaxo-text-secondary hover:text-plaxo-text-primary"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
        
        <p className="text-plaxo-text-secondary mb-6">
          {message}
        </p>
        
        <div className="flex gap-3 justify-end">
          <button
            onClick={onCancel}
            className="px-4 py-2 text-plaxo-text-secondary hover:text-plaxo-text-primary border border-plaxo-border rounded-md hover:bg-plaxo-hover transition-colors"
          >
            {cancelText}
          </button>
          <button
            onClick={onConfirm}
            className={`px-4 py-2 text-white rounded-md transition-colors ${
              variant === 'danger' 
                ? 'bg-red-600 hover:bg-red-700' 
                : 'bg-yellow-600 hover:bg-yellow-700'
            }`}
          >
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
