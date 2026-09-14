import { useState } from 'react';
import { Upload, X, Loader2, AlertCircle, CheckCircle, FileText } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';

interface Props {
  onClose: () => void;
  onImportComplete: () => void;
}

export default function ImportModal({ onClose, onImportComplete }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [dragOver, setDragOver] = useState(false);

  const handleFileSelect = async (file: File) => {
    if (!file.name.endsWith('.2fas')) {
      setError('Por favor, selecione um arquivo .2fas válido');
      return;
    }

    setLoading(true);
    setError('');
    setSuccess('');

    try {
      const content = await file.text();
      const importedCount = await invoke<number>('import_2fas_file', { fileContent: content });
      
      setSuccess(`${importedCount} aplicativo(s) importado(s) com sucesso!`);
      setTimeout(() => {
        onImportComplete();
        onClose();
      }, 2000);
    } catch (err) {
      setError(err as string || 'Erro ao importar arquivo');
    } finally {
      setLoading(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    
    const files = Array.from(e.dataTransfer.files);
    if (files.length > 0) {
      handleFileSelect(files[0]);
    }
  };

  const handleFileInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      handleFileSelect(files[0]);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 z-50" onClick={onClose}>
      <div className="bg-plaxo-surface border border-plaxo-border rounded-2xl p-6 w-full max-w-md shadow-2xl" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div className="flex items-center justify-center w-10 h-10 bg-plaxo-primary/10 rounded-xl">
              <Upload className="w-5 h-5 text-plaxo-primary" />
            </div>
            <h2 className="text-lg font-heading font-semibold text-plaxo-text">
              Importar do 2FAS
            </h2>
          </div>
          <button
            onClick={onClose}
            className="text-plaxo-text-secondary hover:text-plaxo-text p-1.5 rounded-lg hover:bg-plaxo-background/50 transition-colors"
            disabled={loading}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="space-y-4">
          <div
            className={`relative border-2 border-dashed rounded-xl p-8 text-center transition-colors ${
              dragOver 
                ? 'border-plaxo-primary bg-plaxo-primary/5' 
                : 'border-plaxo-border hover:border-plaxo-primary/50'
            }`}
            onDrop={handleDrop}
            onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
            onDragLeave={() => setDragOver(false)}
          >
            {loading ? (
              <div className="space-y-3">
                <Loader2 className="w-8 h-8 text-plaxo-primary mx-auto animate-spin" />
                <p className="text-plaxo-text">Importando arquivo...</p>
              </div>
            ) : (
              <div className="space-y-3">
                <FileText className="w-8 h-8 text-plaxo-text-secondary mx-auto" />
                <div>
                  <p className="text-plaxo-text font-medium mb-1">
                    Arraste seu arquivo .2fas aqui
                  </p>
                  <p className="text-sm text-plaxo-text-secondary">
                    ou clique para selecionar
                  </p>
                </div>
                <input
                  type="file"
                  accept=".2fas"
                  onChange={handleFileInput}
                  className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                  disabled={loading}
                />
              </div>
            )}
          </div>

          {error && (
            <div className="flex items-center gap-2 text-plaxo-error text-sm bg-plaxo-error/10 px-3 py-2 rounded-lg border border-plaxo-error/20">
              <AlertCircle className="w-4 h-4" />
              {error}
            </div>
          )}

          {success && (
            <div className="flex items-center gap-2 text-plaxo-success text-sm bg-plaxo-success/10 px-3 py-2 rounded-lg border border-plaxo-success/20">
              <CheckCircle className="w-4 h-4" />
              {success}
            </div>
          )}

          <div className="text-xs text-plaxo-text-secondary bg-plaxo-background/30 p-3 rounded-lg">
            <p className="font-medium mb-1">Como exportar do 2FAS:</p>
            <p>1. Abra o app 2FAS</p>
            <p>2. Vá em Configurações → Backup</p>
            <p>3. Toque em "Exportar" e salve o arquivo .2fas</p>
          </div>
        </div>
      </div>
    </div>
  );
}
