import { useState } from 'react';
import { Shield, Eye, EyeOff, Loader2, RotateCcw } from 'lucide-react';

interface Props {
  onSubmit: (password: string) => Promise<boolean>;
  onReset?: () => void;
  isFirstTime: boolean;
}

export default function MasterPasswordModal({ onSubmit, onReset, isFirstTime }: Props) {
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [showPassword, setShowPassword] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!password.trim()) return;

    setLoading(true);
    setError('');

    const isValid = await onSubmit(password);
    if (!isValid) {
      setError('Senha inválida');
    }

    setLoading(false);
  };

  return (
    <div className="auth-card">
      <div className="text-center mb-8">
        <div className="inline-flex items-center justify-center w-16 h-16 bg-plaxo-primary/10 rounded-2xl mb-4">
          <Shield className="w-8 h-8 text-plaxo-primary" />
        </div>
        <h1 className="text-2xl font-heading font-bold text-plaxo-text mb-2">
          Plaxo OTP
        </h1>
        <p className="text-plaxo-text-secondary text-sm">
          {isFirstTime ? 'Configure sua senha mestre para proteger seus dados' : 'Digite sua senha mestre para continuar'}
        </p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-6">
        <div className="space-y-2">
          <label className="text-sm font-medium text-plaxo-text">
            Senha Mestre
          </label>
          <div className="relative">
            <input
              type={showPassword ? 'text' : 'password'}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Digite sua senha mestre"
              className="w-full px-4 py-3 pr-12 bg-plaxo-background/50 border border-plaxo-border rounded-xl text-plaxo-text placeholder-plaxo-text-secondary focus:outline-none focus:border-plaxo-primary focus:ring-2 focus:ring-plaxo-primary/20 transition-all [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden"
              disabled={loading}
              autoComplete="current-password"
            />
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-plaxo-text-secondary hover:text-plaxo-text transition-colors z-10"
              disabled={loading}
              tabIndex={-1}
            >
              {showPassword ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
            </button>
          </div>
          {error && (
            <div className="flex items-center gap-2 text-plaxo-error text-sm bg-plaxo-error/10 px-3 py-2 rounded-lg border border-plaxo-error/20">
              <div className="w-1 h-4 bg-plaxo-error rounded-full" />
              {error}
            </div>
          )}
        </div>

        <button
          type="submit"
          disabled={loading || !password.trim()}
          className="w-full flex items-center justify-center gap-2 py-3 bg-plaxo-primary hover:bg-plaxo-primary-hover text-plaxo-background font-semibold rounded-xl transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {loading ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Verificando...
            </>
          ) : (
            <>
              <Shield className="w-4 h-4" />
              {isFirstTime ? 'Criar Senha Mestre' : 'Acessar Aplicação'}
            </>
          )}
        </button>

        {!isFirstTime && onReset && (
          <button
            type="button"
            onClick={onReset}
            className="w-full flex items-center justify-center gap-2 py-2 text-plaxo-text-secondary hover:text-red-500 transition-colors text-sm"
            disabled={loading}
          >
            <RotateCcw className="w-4 h-4" />
            Resetar tudo e criar nova senha
          </button>
        )}
      </form>
    </div>
  );
}
