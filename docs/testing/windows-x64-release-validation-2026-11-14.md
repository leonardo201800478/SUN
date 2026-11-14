# Validação Windows x64 — build Release do desktop

**Data:** 2026-11-14
**Repositório:** https://github.com/leonardo201800478/sega-emulator
**Branch validada:** `main`
**Commit validado:** `118b96bf411248173b6215b7bf16aebdf3a66bff`
**Escopo:** validação local de formatação, Clippy, testes automatizados e build Release para Windows x64/MSVC.
**Ambiente reportado:** Windows 11; Rust `1.99.1 (58cf8f9c3 2026-10-13)`; Cargo `1.99.1 (5f94df478 2026-08-27)`; target `x86_64-pc-windows-msvc`.

> Esta documentação registra os comandos e resultados fornecidos pelo usuário no ambiente Windows local. Não representa execução feita pelo CI nem pelo assistente. A validação física da aplicação continua separada dos testes automatizados.

## 1. Baseline e integridade do checkout

- A branch local era `main`, sem commits locais à frente de `origin/main`.
- O checkout foi atualizado por fast-forward, sem rebase, reset ou descarte de alterações.
- SHA local e remoto após a atualização: `118b96bf411248173b6215b7bf16aebdf3a66bff`.
- `git status --short --branch` reportou `## main...origin/main`, sem alterações locais.
- Target Rust instalado: `x86_64-pc-windows-msvc`.

## 2. Desktop Slint / WGPU 30

Manifesto: `native/sega_desktop/Cargo.toml`.

Comandos executados:

```powershell
cargo fmt --manifest-path native/sega_desktop/Cargo.toml -- --check

cargo clippy --release --locked --all-targets --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc -- -D warnings

cargo test --release --locked --all-targets --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc

cargo build --release --locked --manifest-path native/sega_desktop/Cargo.toml --target x86_64-pc-windows-msvc
```

| Gate | Resultado | Evidência |
|---|---|---|
| rustfmt | PASS | `--check` terminou sem diferenças reportadas |
| Clippy Release | PASS | Finalizou sem warnings com `-D warnings` |
| Testes Release | PASS | 26 aprovados; 0 falhas; 0 ignorados |
| Build Release | PASS | Compilação finalizada sem erro |

Os testes do host cobrem, entre outros pontos, telemetria e controles de áudio, volume/mute, fullscreen, teclado, carregamento de ROM ZIP, conversão RGB888/stride, frame pacing e publicação de frames completos do VDP SMS para o mailbox do desktop.

Dependências relevantes observadas no grafo compilado: Slint `1.18.1`, WGPU `30.0.1`, CPAL `0.18.2` e crates internos de runtime, áudio, vídeo e observabilidade.

## 3. Saída de áudio — crate CPAL

Manifesto independente: `native/audio_output/Cargo.toml`.

Comandos executados:

```powershell
cargo fmt --manifest-path native/audio_output/Cargo.toml -- --check

cargo clippy --release --locked --all-targets --manifest-path native/audio_output/Cargo.toml --target x86_64-pc-windows-msvc -- -D warnings

cargo test --release --locked --all-targets --manifest-path native/audio_output/Cargo.toml --target x86_64-pc-windows-msvc
```

| Gate | Resultado | Evidência |
|---|---|---|
| rustfmt | PASS | `--check` terminou sem diferenças reportadas |
| Clippy Release | PASS | Finalizou sem warnings com `-D warnings` |
| Testes Release | PASS | 21 aprovados; 0 falhas; 0 ignorados |

Cobertura reportada pelos testes: conversão PCM e balanceamento; precisão de amostras normalizadas; FIFO SPSC, ordenação e epochs; contabilização de underflow/overflow; desconexão de produtor/consumidor; negociação F32/I24/I16 e taxas 48/88,2/96 kHz; fallback PCM16; métricas iniciais sem abertura de dispositivo; identificação de dispositivos quando a enumeração é bem-sucedida.

O crate CPAL foi compilado e testado como unidade independente. Isso não demonstra, isoladamente, que o stream físico foi aberto pelo desktop, que o endpoint reproduziu áudio ou que houve estabilidade prolongada.

## 4. Artefato executável

Verificação local reportada:

```powershell
$exe = ".\target\x86_64-pc-windows-msvc\release\sega-desktop.exe"

if (Test-Path $exe) {
    Get-Item $exe | Format-List FullName, Length, LastWriteTime
} else {
    Write-Error "Executável não encontrado."
}
```

Resultado:

- Caminho: `target\x86_64-pc-windows-msvc\release\sega-desktop.exe`
- Tamanho: `22.192.640 bytes` (aproximadamente 21,15 MiB).
- Última gravação reportada: `14/11/2026 11:35:18`.

O arquivo foi encontrado no caminho esperado. SHA-256, inspeção de DLLs/imports, execução da aplicação e empacotamento portátil ainda não foram registrados nesta etapa.

## 5. Classificação consolidada

| Área | Estado | Limite da evidência |
|---|---|---|
| Checkout `main` no SHA declarado | PASS | Estado local limpo reportado |
| Formatação desktop e áudio | PASS | `cargo fmt --check` |
| Clippy desktop e áudio | PASS | Release, todos os targets, `-D warnings` |
| Testes automatizados desktop | PASS | 26/26 |
| Testes automatizados CPAL | PASS | 21/21 |
| Build Release desktop Windows x64 | PASS | Cargo concluiu sem erro |
| Existência do executável | PASS | `Test-Path` e metadados reportados |
| Inicialização real da janela | NOT RUN | Ainda não há resultado de execução |
| Seleção e identificação física de Vulkan | NOT RUN | Build não comprova backend ativo |
| Carregamento e gameplay de ROM SMS/SG-1000 | NOT RUN | Testes unitários não substituem sessão real |
| Apresentação contínua de frames em monitor | NOT RUN | Necessita execução física |
| Abertura e reprodução em endpoint CPAL | NOT RUN | Testes do crate não abrem stream físico |
| Underruns, estabilidade prolongada e hotplug | NOT RUN | Campanha física pendente |
| SHA-256 e dependências do executável | NOT RUN | Auditoria do binário pendente |
| ZIP portátil e distribuição | NOT RUN | Não empacotar antes da auditoria de dependências |

## 6. Próximo protocolo de aceite físico

Executar no mesmo Windows x64 que produziu o binário:

1. Iniciar `sega-desktop.exe` e registrar se a janela abre, permanece responsiva e encerra sem panic.
2. Registrar adapter, backend gráfico efetivo e mensagens de inicialização WGPU/Vulkan.
3. Carregar uma ROM SMS obtida legalmente; registrar região, identidade/hash da ROM e comportamento de substituição.
4. Observar frames reais, proporção, resize, pausa, retomada, reset e controles durante gameplay.
5. Confirmar áudio audível no endpoint selecionado; registrar formato, taxa, canais, volume, mute, underruns e erros.
6. Manter uma sessão prolongada e guardar logs/telemetria. Comparar execução com áudio ativo e, quando necessário, modo video-only, sem atribuir causalidade ao Scheduler antes de analisar os dados.
7. Calcular SHA-256 do executável e inspecionar dependências dinâmicas antes de criar pacote portátil.

A GUI não pode avançar ciclos nem controlar o MachineScheduler. Renderização, callback de áudio e apresentação física não podem alterar a temporização do runtime. Não introduzir sleeps, frame skipping, throttling ou clocks na GUI para mascarar problemas de pacing.

## 7. Conclusão

A validação automatizada local do host desktop e do crate de saída CPAL está aprovada no commit `118b96bf411248173b6215b7bf16aebdf3a66bff`: formatação, Clippy sem warnings, 47 testes aprovados e build Release Windows x64 concluído. O executável foi localizado no caminho esperado.

**O aceite de execução física permanece aberto.** Não declarar R2/R3 integralmente aceitos, nem publicar pacote portátil, até concluir os testes reais de janela, Vulkan, ROM, frames, gameplay e áudio físico descritos acima.
