/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
    "./pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        display: ["var(--font-display)", "system-ui", "sans-serif"],
        body: ["var(--font-body)", "system-ui", "sans-serif"],
      },
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        neon: {
          green: "#00FF87",
          violet: "#7B61FF",
          cyan: "#00D4FF",
        },
        surface: {
          1: "#0A0A0C",
          2: "#111113",
          3: "#18181B",
        },
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
      },
      borderColor: {
        DEFAULT: "hsl(var(--border))",
      },
      animation: {
        "aurora": "aurora 15s ease-in-out infinite",
        "pulse-glow": "pulse-glow 3s ease-in-out infinite",
        "float": "float 6s ease-in-out infinite",
        "shimmer": "shimmer 4s linear infinite",
        "neon-pulse": "neon-pulse 2s ease-in-out infinite",
        "slide-up": "slide-up 0.6s ease-out",
        "glow-line": "glow-line 3s linear infinite",
        "wave": "wave 12s ease-in-out infinite",
      },
      boxShadow: {
        "glow-sm": "0 0 10px rgba(0, 255, 135, 0.2)",
        "glow": "0 0 20px rgba(0, 255, 135, 0.3), 0 0 60px rgba(0, 255, 135, 0.1)",
        "glow-lg": "0 0 30px rgba(0, 255, 135, 0.4), 0 0 80px rgba(0, 255, 135, 0.2)",
        "glow-violet": "0 0 20px rgba(123, 97, 255, 0.3), 0 0 60px rgba(123, 97, 255, 0.1)",
        "glow-cyan": "0 0 20px rgba(0, 212, 255, 0.3), 0 0 60px rgba(0, 212, 255, 0.1)",
        "inner-glow": "inset 0 1px 0 0 rgba(255, 255, 255, 0.05)",
      },
      backgroundImage: {
        "gradient-radial": "radial-gradient(var(--tw-gradient-stops))",
        "aurora-gradient": "linear-gradient(135deg, rgba(0,255,135,0.15), rgba(123,97,255,0.15), rgba(0,212,255,0.15))",
      },
    },
  },
  plugins: [],
}
