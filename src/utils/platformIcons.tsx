import { 
  Github, 
  Mail, 
  Chrome, 
  Gamepad2, 
  MessageCircle, 
  Music, 
  ShoppingCart, 
  Cloud, 
  Database, 
  Shield,
  Smartphone,
  Globe,
  Key
} from 'lucide-react';

const platformIcons: Record<string, React.ComponentType<any>> = {
  // Desenvolvimento
  'github': Github,
  'gitlab': Database,
  'bitbucket': Database,
  
  // Google Services
  'google': Chrome,
  'gmail': Mail,
  'drive': Cloud,
  'youtube': Music,
  
  // Social
  'discord': MessageCircle,
  'telegram': MessageCircle,
  'whatsapp': MessageCircle,
  'twitter': MessageCircle,
  'facebook': Globe,
  'instagram': Smartphone,
  'linkedin': Globe,
  
  // Gaming
  'steam': Gamepad2,
  'epic': Gamepad2,
  'battle.net': Gamepad2,
  'origin': Gamepad2,
  
  // E-commerce
  'amazon': ShoppingCart,
  'paypal': ShoppingCart,
  'mercadolivre': ShoppingCart,
  
  // Cloud/Hosting
  'aws': Cloud,
  'azure': Cloud,
  'digitalocean': Cloud,
  'heroku': Cloud,
  'vercel': Cloud,
  'netlify': Cloud,
  
  // Crypto
  'binance': Database,
  'coinbase': Database,
  'metamask': Key,
  
  // Microsoft
  'microsoft': Globe,
  'outlook': Mail,
  'office': Globe,
  
  // Apple
  'apple': Smartphone,
  'icloud': Cloud,
  
  // Default
  'default': Shield
};

export function getPlatformIcon(appName: string) {
  const name = appName.toLowerCase();
  
  // Busca por correspondência exata primeiro
  if (platformIcons[name]) {
    return platformIcons[name];
  }
  
  // Busca por palavras-chave
  for (const [key, icon] of Object.entries(platformIcons)) {
    if (name.includes(key) || key.includes(name)) {
      return icon;
    }
  }
  
  return platformIcons.default;
}

export function getPlatformColor(appName: string): string {
  const name = appName.toLowerCase();
  
  const colors: Record<string, string> = {
    'github': '#24292e',
    'google': '#4285f4',
    'gmail': '#ea4335',
    'discord': '#5865f2',
    'steam': '#1b2838',
    'amazon': '#ff9900',
    'microsoft': '#00a1f1',
    'apple': '#000000',
    'facebook': '#1877f2',
    'twitter': '#1da1f2',
    'instagram': '#e4405f',
    'linkedin': '#0077b5',
    'youtube': '#ff0000',
    'spotify': '#1db954',
    'netflix': '#e50914',
    'default': '#667eea'
  };
  
  for (const [key, color] of Object.entries(colors)) {
    if (name.includes(key) || key.includes(name)) {
      return color;
    }
  }
  
  return colors.default;
}
