
# The purpose of this Makefile is mostly as a repository of cmds to reduce head-scratching when revisiting after extended absences!

dev:
	pusd src-tauri && cargo tauri dev && popd

run:
	pushd src-tauri && cargo tauri dev && popd

build:
	pushd src-tauri && cargo tauri build && popd

lcheck:
	pushd src-tauri && cargo lcheck && popd

lcw:
	pushd && npx nodemon --watch src -e rs --exec cargo lcheck && popd

sjs:
	.\\mill.bat switche.webapp

sjsw:
	.\\mill.bat -w switche.webapp

tabs:
	cat -e -t -v Makefile



.DEFAULT_GOAL := lcheck

