import { Search, Plus, Upload } from 'lucide-react';

interface Props {
  value: string;
  onChange: (value: string) => void;
  onAddClick: () => void;
  onImportClick: () => void;
}

export default function SearchBar({ value, onChange, onAddClick, onImportClick }: Props) {
  return (
    <div className="space-y-3">
      <div className="flex gap-3">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-plaxo-text-secondary" />
          <input
            type="text"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder="Pesquisar aplicativos..."
            className="w-full pl-10 pr-4 py-2.5 bg-plaxo-background/50 border border-plaxo-border rounded-xl text-plaxo-text placeholder-plaxo-text-secondary focus:outline-none focus:border-plaxo-primary focus:ring-2 focus:ring-plaxo-primary/20 transition-all text-sm"
          />
        </div>
        <button
          onClick={onAddClick}
          className="flex items-center justify-center w-10 h-10 bg-plaxo-primary hover:bg-plaxo-primary-hover text-plaxo-background rounded-xl transition-colors"
          title="Adicionar aplicativo"
        >
          <Plus className="w-4 h-4" />
        </button>
      </div>
      
      <button
        onClick={onImportClick}
        className="w-full flex items-center justify-center gap-2 py-2.5 bg-plaxo-background/30 hover:bg-plaxo-background/50 text-plaxo-text border border-plaxo-border rounded-xl transition-colors text-sm font-medium"
      >
        <Upload className="w-4 h-4" />
        Importar do 2FAS
      </button>
    </div>
  );
}
