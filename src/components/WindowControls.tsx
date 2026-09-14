import { appWindow } from '@tauri-apps/api/window';
import { Minus } from 'lucide-react';

export default function WindowControls() {
  const minimize = () => appWindow.hide();

  return (
    <div className="absolute top-4 right-4 z-50">
      <button 
        className="flex items-center justify-center w-8 h-8 text-plaxo-text-secondary hover:text-plaxo-text hover:bg-plaxo-background/50 rounded-lg transition-colors" 
        onClick={minimize} 
        title="Minimizar para bandeja"
      >
        <Minus className="w-4 h-4" />
      </button>
    </div>
  );
}
