@echo off
echo === Teste do DDS Converter CLI em Rust ===
echo.

echo === 1. Teste com arquivo que falha ===
echo Testando com continue-on-error ativado...
echo.

.\target\release\dds-converter.exe ^
    --input "../extract/lyshineui/images/icons/worldmap" ^
    --output "../test-rust-output" ^
    --verbose ^
    --continue-on-error ^
    --concurrency 2

echo.
echo === 2. Informacoes do executavel ===
echo Tamanho:
dir /b .\target\release\dds-converter.exe | for %%i in (.\target\release\dds-converter.exe) do echo %%~zi bytes
echo.
echo === 3. Teste texconv embutido ===
echo O texconv.exe esta embutido no executavel!
echo Nao precisa de arquivos externos.
echo.

pause
