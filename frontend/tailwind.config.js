/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        brand: {
          50: '#fff4ee',
          100: '#ffe2d2',
          200: '#ffc2a7',
          300: '#ff9a72',
          400: '#ff6b35',
          500: '#e8541f',
          600: '#bf3e12',
          700: '#962f0f',
          800: '#76280f',
          900: '#5f220f'
        }
      },
      boxShadow: {
        glow: '0 0 0 1px rgba(255, 107, 53, 0.25), 0 8px 24px rgba(0, 0, 0, 0.24)'
      }
    }
  },
  plugins: []
};
