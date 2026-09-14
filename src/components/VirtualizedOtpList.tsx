import { useMemo } from 'react';
import OtpItem from './OtpItem';
import { PERFORMANCE_CONFIG } from '../config/performance';

interface OtpApp {
  id: string;
  name: string;
  secret: string;
}

interface Props {
  apps: OtpApp[];
  onDelete: (id: string) => void;
  onEdit: () => void;
  isVisible?: boolean;
}

export default function VirtualizedOtpList({ apps, onDelete, onEdit, isVisible = true }: Props) {
  const visibleApps = useMemo(() => {
    const maxApps = isVisible ? PERFORMANCE_CONFIG.MAX_VISIBLE_APPS : 5;
    return apps.slice(0, maxApps);
  }, [apps, isVisible]);

  const hasMoreApps = apps.length > visibleApps.length;

  return (
    <div className="max-w-md mx-auto space-y-4">
      {visibleApps.map(app => (
        <OtpItem
          key={app.id}
          app={app}
          onDelete={onDelete}
          onEdit={onEdit}
          isVisible={isVisible}
        />
      ))}

      {hasMoreApps && (
        <div className="text-center text-plaxo-text-secondary text-sm py-4">
          Mostrando {visibleApps.length} de {apps.length} aplicativos.
          <br />
          {!isVisible ? "App minimizado - mostrando menos itens." : "Use a busca para encontrar aplicativos específicos."}
        </div>
      )}
    </div>
  );
}
