/** @type {import('tailwindcss').Config} */
module.exports = {
    mode: "all",
    content: ["./src/**/*.{rs,html,css}", "./dist/**/*.html"],
    theme: {
        extend: {},
    },
    plugins: [
        require("daisyui")
    ],
    daisyui: {
        themes: ["dark", "cupcake", "night"]
    }
};
