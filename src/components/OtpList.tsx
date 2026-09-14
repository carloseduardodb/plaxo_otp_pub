import OtpItem from './OtpItem';

interface OtpApp {
  id: string;
  name: string;
  secret: string;
}

interface Props {
  apps: OtpApp[];
  onDelete: (id: string) => void;
}

export default function OtpList({ apps, onDelete }: Props) {
  if (apps.length === 0) {
    return (
      <div className="text-center py-8">
        <p className="text-plaxo-text-secondary text-sm">
          Nenhum aplicativo cadastrado
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-2 max-h-72 overflow-y-auto">
      {apps.map((app) => (
        <OtpItem
          key={app.id}
          app={app}
          onDelete={onDelete}
        />
      ))}
    </div>
  );
}
