# Prompt de continuidade — Sega Emulator / Correções SMS1

Copie o texto abaixo para iniciar uma nova conversa.

---

Atue como Arquiteto de Software, Engenheiro de Emulação e Desenvolvedor Principal do projeto **Sega Emulator — Emulação de Alta Precisão**.

**Repositório:** https://github.com/leonardo201800478/sega-emulator
**Referência técnica:** https://www.smspower.org/
**Baseline registrada:** `main` = `118b96bf411248173b6215b7bf16aebdf3a66bff` (14/11/2026). É uma SHA histórica: antes de qualquer ação, consulte o HEAD atual da main, PRs abertas e CI.

## Objetivo imediato
Prosseguir do plano oficial `docs/roadmap/2026-11-14-sms-emulation-correction-plan.md`, iniciando P1 — portas de dados e endereçamento VRAM do VDP SMS1. Não reiniciar tarefas concluídas nem fazer correções cosméticas. Identificar causa raiz com testes discriminantes.

## Leitura obrigatória
1. Plano de correção SMS.
2. `docs/sms1-vdp-completion-scorecard.md`.
3. `docs/audits/sms1-vdp-temporal-evidence-matrix-2026-11-14.md`.
4. `docs/audits/sms1-vdp-software-scope-2026-11-14.md`, se existir.
5. `docs/z80-completion-scorecard.md`.
6. `docs/z80-dd-fd-semantic-matrix.md`.
7. PRs #850–#853: verificar status, head SHA, reviews e CI atuais.

## Sequência
- P0 baseline, checkout, stashes, PRs e CI; preservar trabalho local.
- P1 reproduzir separadamente `1byte wr sets rd adr` e `1byte wr sets wr adr`; testar command latch, endereço read/write, read-ahead, incremento/wrap e snapshot. Corrigir só depois de isolar.
- P2 Line IRQ/VCount, HCounter, HINT/R10, VINT e integração Scheduler/Z80.
- P3 Scroll Test, quatro faixas, planos sobrepostos, direção e raster writes.
- P4 Zoom Effect e distorção à direita; comparar framebuffer lógico antes do WGPU.
- P5 SAT, sprites, atributos, padrões, prioridade, clipping, overflow, colisão e fetch pipeline.
- P6 modos SMS1/legacy-TMS, mapper e compatibilidade regional; preservar SMS2 e SG-1000.
- P7 boot de Andre Agassi e trace Z80/mapper/VDP/PSG.
- P8 diagnósticos externos com oracle explícito.
- P9 regressões, Windows x64 Release, scorecards e liberação.

## Sintomas observados
- SMS VDP Test v1.31: divergências exclusivas em `1byte wr sets rd adr`, `1byte wr sets wr adr` e `Line IRQ VCount`.
- Scroll Test: primeira faixa parece correta; faixas seguintes divergem na composição/overlays e há sentido vertical invertido na quarta faixa.
- Zoom Effect: leve distorção de linha na extremidade direita.
- ZEXALL V0.12VDP: tela preta no Sega Emulator; Ares exibe os resultados.
- ZEXALL V0.12SDSC: tela preta no Sega Emulator; Ares exibe o logo de boot.
- Andre Agassi Tennis: tela preta e zunido; o log de host confirmou Vulkan/CPAL e pacing aproximado de 60 Hz, mas não continha PC Z80 nem trace suficiente para localizar a causa.
- Cosmic Spacehead: sprites deslocados; considerar mapper Codemasters e requisito de vídeo especial/SMS2.
- The Smurfs Travel the World (Brazil): sprites deslocados; identidade do dump precisa de confirmação.
- A campanha física/manual foi pausada pelo usuário até progresso interno verificável. Não pedir novas capturas/ROMs sem justificar que os gates internos foram concluídos.

## Arquitetura inegociável
- Backend 100% Rust.
- Slint 1.18 GUI, WGPU 30 renderização, Vulkan inicial no Windows x64, CPAL 0.18.2 áudio; SDL3 somente após auditoria.
- MachineScheduler é a única autoridade temporal.
- GUI não implementa hardware, CPU, ciclos ou temporização.
- Preservar Z80, Scheduler, SMS/SG-1000, VDPs, PSG, barramentos, memórias, mappers e testes validados.
- Não criar callbacks por instrução para a GUI nem corrigir hardware na camada WGPU.

## Política Z80
Consultar scorecards antes de tocar no Z80. E7 permanece IN_PROGRESS; E8/ZEXDOC/ZEXALL está BLOCKED. Não iniciar E8 antes de fechamento demonstrável de E7. Não reabrir E4C/E5/E6 sem regressão reproduzível.

## Agentes
Usar perfis em `.github/agents/`:
- `sms-vdp-data-port`: P1.
- `sms-vdp-timing`: P2.
- `sms-vdp-raster`: P3–P5.
- `sms-regression-auditor`: revisão independente.

Os arquivos são definições de agentes customizados para runtimes compatíveis (GitHub Copilot/VS Code); sua presença no repositório não significa que execuções autônomas já foram iniciadas. Use o runtime de agentes disponível para delegar, mantendo apenas um implementador por área/arquivos compartilhados.

## Processo de engenharia
1. Antes do patch, declarar hipótese e teste discriminante.
2. Escrever teste que reproduz o defeito na baseline.
3. Fazer a menor correção possível.
4. Executar testes focados, regressões de perfil, fmt, Clippy e workspace aplicável.
5. Fazer revisão independente.
6. Abrir PR pequena; aguardar CI no HEAD exato e corrigir falhas.
7. Atualizar plano, scorecard e matriz.
8. Integrar à main somente com gates técnicos e CI verdes.
9. Atualizar este handoff com nova SHA, PRs e estado real.

## Próxima tarefa concreta
Leia a implementação atual das portas de dados/controle do SMS VDP e os testes existentes. Identifique o caminho runtime desktop SMS1 até `sega_video::sms::SmsVdp`. Construa dois testes independentes para os dois casos de endereço do data port. Não alterar produção até isolar as sequências de falha. Entregue análise, testes, correção, regressões e PR.

**Resultado esperado:** progresso mensurável no runtime, não apenas documentação ou imagens.
