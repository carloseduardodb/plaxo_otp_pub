/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        plaxo: {
          background: '#0F172A',
          surface: '#1E293B',
          primary: '#A3E635',
          'primary-hover': '#84CC16',
          text: '#F8FAFC',
          'text-secondary': '#CBD5E1',
          border: '#334155',
          success: '#22C55E',
          error: '#EF4444',
          warning: '#EAB308',
        }
      },
      fontFamily: {
        sans: ['Inter', 'sans-serif'],
        heading: ['Outfit', 'sans-serif'],
      },
      boxShadow: {
        soft: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
        focus: '0 0 0 3px rgba(163, 230, 53, 0.3)',
      }
    },
  },
  plugins: [],
}
