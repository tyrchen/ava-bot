run: build-css
	@RUST_LOG=info cargo run

watch:
	@watchexec --restart --exts rs,js,css,j2 --ignore public -- make run

build-css:
	@echo "Building CSS..."
	@npx tailwindcss build -i input.css -o public/css/main.css --minify
