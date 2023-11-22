/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.rs",
    "./templates/**/*.html.j2",
    "./public/**/*.{html,js,css}",
  ],
  theme: {
    extend: {
      width: {
        128: "32rem",
        192: "48rem",
        256: "64rem",
      },
      height: {
        128: "32rem",
        192: "48rem",
        256: "64rem",
      },
    },
  },
  plugins: [require("@tailwindcss/typography"), require("@tailwindcss/forms")],
};
