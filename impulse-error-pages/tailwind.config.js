module.exports = {
  mode: "all",
  content: [
    "./src/**/*.{rs,html,css}",
    "../dist/**/*.html",
  ],
  theme: {
    extend: {
      keyframes: {},
      animation: {}
    },
    fontFamily: {
      'sans': ['Roboto']
    }
  },
  plugins: [],
}
