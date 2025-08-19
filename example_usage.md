# Exemplo de Uso - New World Tooltip Animations

## Converter sequências PNG em GIF animado

```cmd
dds-converter.exe --animation-mode --input "E:\new-world-tools_0.12.7_windows_amd64\extract\lyshineui\images\tooltip" --output "E:\animations" --animation-format gif --frame-delay 100
```

## Converter sequências PNG em WebP animado

```cmd
dds-converter.exe --animation-mode --input "E:\new-world-tools_0.12.7_windows_amd64\extract\lyshineui\images\tooltip" --output "E:\animations" --animation-format webp --frame-delay 100
```

## Parâmetros disponíveis:

- `--animation-mode`: Ativa o modo de criação de animações
- `--input`: Pasta com os arquivos PNG sequenciais
- `--output`: Pasta onde salvar as animações
- `--animation-format`: Formato da animação (gif ou webp)
- `--frame-delay`: Delay entre frames em milissegundos (padrão: 100ms)
- `--verbose`: Mostra informações detalhadas do processo

## Exemplo com delay personalizado (mais rápido):

```cmd
dds-converter.exe --animation-mode --input "E:\new-world-tools_0.12.7_windows_amd64\extract\lyshineui\images\tooltip" --output "E:\animations" --animation-format gif --frame-delay 50 --verbose
```

O programa automaticamente detecta sequências de PNG baseado no padrão de nomes como:
- `tooltip_001.png`, `tooltip_002.png`, etc.
- `animation_01.png`, `animation_02.png`, etc.
- Qualquer padrão `nome_numero.png`