# SMS Boot Compatibility — Auditoria inicial

**Data:** 2026-11-15  
**Baseline auditada:** `main` — `90036d8d9f988640faf1540d2b4dc1548780c13f`  
**Branch de trabalho:** `fix/sms-boot-compatibility`  
**Status:** B1 — auditoria estática inicial concluída; reprodução de ROMs e validação experimental pendentes.

## 1. Escopo e limites

Esta auditoria não altera o Z80 compartilhado, VDP, PSG ou Scheduler. Os valores descritos como “iniciais” são valores efetivamente atribuídos pelo código na construção da máquina; não são automaticamente valores físicos universais de power-on.

A baseline `main` foi confirmada no SHA informado. A branch de trabalho foi criada diretamente desse commit. Nenhuma correção de runtime foi aplicada nesta etapa.

## 2. Caminhos de construção e inicialização encontrados

### 2.1 Máquina temporal

Os construtores públicos de `SmsTemporalMachine` convergem para `new_with_hardware_profile_and_sram_size`. O construtor inicializa:

- `cpu: Z80MicroSequencer::default()`;
- `bus: SmsTemporalBus::new_with_hardware_profile_and_sram_size(...)`;
- estado de interrupção externa e continuação sem transação pendente;
- rastreamento desabilitado.

O construtor temporal do barramento cria `SmsMemory::with_cartridge_sram_size(...)`, `SmsTemporalCoordinator::new(...)`, `ScheduledPsg` com perfil SMS, `SmsIo::new(territory)` e `SmsVdp::new_with_profile(profile)`.

**Resultado confirmado:** no SHA auditado, o caminho temporal público não recebe BIOS. O suporte opcional a BIOS existe em `SmsMemory::with_bios`, mas não é usado pelo construtor temporal da baseline.

### 2.2 Worker/runtime e firmware no frontend

`SmsTemporalWorker::new` e os construtores de perfil chamam o builder temporal sem BIOS. A auditoria do host Slint confirmou que a UI já persiste `firmware_directory`, seleções regionais e `firmware_boot_enabled`, mas o valor padrão é `false`. O próprio texto da tela informa que a varredura identifica arquivos, porém não comprova boot, e que a inicialização via BIOS só será habilitada após integrar o dispositivo interno e seu mapa específico ao runtime temporal.

`scan_sms_firmware_directory` consulta `SmsFirmwareCatalog` para identificar e listar firmware; isso não encaminha bytes para a máquina. O factory `create_sms_worker` em `native/sega_desktop/src/main.rs` chama `SmsTemporalWorker::new_with_region_preference_and_hardware_profile_and_cartridge_sram_size` com ROM, região, revisão do VDP e SRAM, sem BIOS. Portanto, o comportamento atual é intencionalmente boot direto; a preferência de firmware é preparatória e não é um toggle de boot funcional na baseline.

Há uma divergência documental relevante: `docs/architecture/sms-bios-firmware-loading.md` ainda declara que o encaminhamento Qt/PyO3 e o boot pelo core Rust estão implementados. Esse documento descreve uma arquitetura anterior (Qt/PyO3/NativeSMSTemporalMachine) e não corresponde ao chamador Slint/Rust temporal atual. Para esta auditoria, o código efetivo de `native/sega_desktop/src/main.rs` e a mensagem explícita da UI prevalecem; o status IMPLEMENTED do documento antigo não comprova boot temporal funcional.

- `Pause`: interrompe a execução e suspende/limpa a saída de áudio; preserva a máquina.
- `Stop`: reconstrói a máquina inteira, preservando o conteúdo e a capacidade de SRAM; é tratado no código como power-off/cold reset.
- `Reset`: também reconstrói a máquina inteira, preservando SRAM.
- `LoadRom`: cria uma nova máquina com a ROM recebida; a implementação atual rejeita substituição via esse comando quando há SRAM no cartucho atual.
- `SaveState`: exporta `SmsTemporalMachineState`; versão do estado do worker = 1. A máquina mantém sua própria versão de estado. `LoadState` exige versão, região, identidade da ROM e perfil de hardware compatíveis.

O reset público do worker não é a mesma operação que aplicar o pino RESET somente ao Z80. A reconstrução também reinicializa memória, VDP, PSG, I/O e timeline.

### 2.3 Diagramas de sequência — baseline atual

**Power-on sem BIOS (caminho efetivamente usado pela máquina temporal):**

```text
Host/Worker
  -> SmsTemporalMachine::new*()
     -> Z80MicroSequencer::default()
     -> SmsTemporalBus::new_with_hardware_profile_and_sram_size()
        -> CartridgeImage::new(ROM)
        -> SmsMemory::with_cartridge_sram_size()
           -> BIOS = None
           -> memory control = $AB
           -> mapper banks = Sega [0,1,2] ou Codemasters [0,1,0]
        -> SmsTemporalCoordinator::new() (tempo inicial zero)
        -> ScheduledPsg::new(PsgProfile::Sms)
        -> SmsIo::new() (I/O control $FF)
        -> SmsVdp::new_with_profile() (estado interno zerado)
  -> primeira busca do Z80 em PC=$0000
```

**Power-on com BIOS — suporte parcial na baseline:**

```text
SmsMemory::with_bios(ROM, BIOS)
  -> valida BIOS de 8 KiB
  -> memory control = $E0
  -> BIOS disponível em $0000-$1FFF enquanto habilitada
  -> cartucho desabilitado pelo latch


Limite: este caminho não está conectado ao construtor de SmsTemporalBus/
SmsTemporalMachine/Worker da baseline main. Não é um caminho de boot temporal
disponível ao frontend neste SHA.
```

**Caminho com BIOS proposto na PR #908 (não integrada):**

```text
Host fornece bytes validados
  -> SmsTemporalWorker::new_with_hardware_profile_bios_...()
     -> SmsTemporalMachine::new_with_hardware_profile_bios_...()
        -> SmsTemporalBus::new_with_hardware_profile_bios_...()
           -> SmsMemory::with_bios_and_sram_size()
  -> Z80 inicia em PC=$0000 e executa instruções da BIOS via Scheduler
  -> BIOS altera o latch de memória (ex.: saída de $AB)
  -> próxima busca passa a observar o cartucho
```

A PR #908 não deve ser tratada como caminho disponível na baseline. Ela é Draft, sua branch divergiu da main atual e seus testes de handoff usam firmware sintético; não comprovam o boot de BIOS comercial real nem a integração do frontend.

**Reset do console solicitado pela aplicação:**

```text
WorkerCommand::Reset
  -> rebuild_sms_temporal_machine_preserving_sram()
     -> nova CPU default
     -> novo barramento/memória/mapper/VDP/PSG/I/O
     -> novo Scheduler em tempo zero
     -> restaura apenas a imagem/capacidade de SRAM
  -> substitui a máquina anterior
```

**Reset do Z80 (operação distinta):**

```text
Z80MicroSequencer::reset()
  -> descarta continuação em andamento
  -> PC, I, R e estado de interrupção/controle são reiniciados
  -> registradores gerais, alternativos, IX/IY e SP são preservados
  -> não reinicializa memória, mapper, VDP, PSG, I/O ou tempo do Scheduler
```

O helper `SmsTemporalMachine::reset_cpu()` é atômico no nível da API; ele não modela uma linha RESET mantida por três ciclos de clock. O manual Zilog UM008011-0816 exige RESET ativo por pelo menos três ciclos completos. Não confundir o helper com uma implementação temporal do pino RESET. O reset do worker, por sua vez, é reconstrução integral da máquina, não o pino RESET isolado.

**Carregamento de outra ROM:**

```text
WorkerCommand::LoadRom(new_rom)
  -> constrói nova SmsTemporalMachine sem BIOS na baseline
  -> substitui ROM e máquina
  -> reinicia estado de CPU, barramento, periféricos e timeline
```

**Save State:**

```text
SaveState -> machine.export_state()
LoadState -> valida versão/ROM/região/perfil -> SmsTemporalMachine::restore_state()
```

## 3. Tabela dos valores iniciais atribuídos pelo código

| Subsistema | Valor/estado na construção temporal da baseline | Classificação |
|---|---|---|
| Z80 PC/SP | `$0000/$0000` | Default do emulador; não é especificação de power-on físico |
| Z80 AF/BC/DE/HL | `$0000` | Default do emulador |
| Z80 AF'/BC'/DE'/HL' | `$0000` | Default do emulador |
| Z80 IX/IY | `$0000/$0000` | Default do emulador |
| Z80 I/R | `$00/$00` | Default do emulador |
| Z80 IFF1/IFF2/IM | `false/false/0` | Default do emulador |
| Z80 WZ/Q/P/HALT | `0/0/false/false` (Q anterior também zero) | Estado interno determinístico do sequenciador |
| Memória BIOS | ausente no caminho temporal atual | Confirmado pelo construtor |
| Memory Control ($3E) | `$AB` sem BIOS | Perfil de boot direto/pós-BIOS escolhido pelo código |
| Mapper Sega | banks `[0,1,2]`, mapper control `$00` | Inicialização do código |
| Mapper Codemasters | banks `[0,1,0]` | Inicialização do código |
| Work RAM | 8 KiB zerados, espelhados em `$C000-$FFFF` | Inicialização do código |
| SRAM de cartucho | ausente, salvo perfil explícito | Política de mídia/hardware |
| VDP registers | 16 bytes zero | Inicialização do código |
| VDP VRAM/CRAM | 16 KiB/32 bytes zero | Inicialização do código |
| VDP status/address/latches | zero; controle sem primeiro byte pendente | Inicialização do código |
| VDP raster | scanline 0, T-state 0, frame 0; IRQ pendente false | Inicialização do código |
| VDP VC/H latch | latch VC = 0; latch HC = 0 | Estado inicial interno, não medida após execução |
| PSG | tone periods `[0,0,0]`; attenuation `[15,15,15,15]`; noise control/latch zero; SMS LFSR seed `$8000`; tone counters `[1,1,1]`; outputs `[1,1,1]`; noise counter resetado para `$10` | Inicialização do core PSG; registradores iniciais coincidem com a observação de Sega integrated PSG, fase interna parcialmente não documentada |
| I/O | control `$FF`; TH externos altos; entradas padrão | Inicialização do código |
| Scheduler | tempo inicial zero; domínios Z80/PSG/VDP registrados | Inicialização do código |

### 3.1 Memory Control $3E

O projeto trata os bits do latch como ativos em nível baixo. `$AB` seleciona o perfil de cartucho sem BIOS: BIOS desabilitada, cartucho habilitado e RAM interna habilitada. `$E0` é o perfil escolhido no construtor de memória com BIOS: BIOS e RAM habilitadas, cartucho/card/expansão desabilitados.

O valor `$E0` aparece no código com referência a SMS Plus GX e notas de Charles MacDonald. Os bits conhecidos de slot/RAM/BIOS coincidem com a descrição de BIOS habilitada, RAM habilitada e slots externos desabilitados; os bits 1–0 são desconhecidos. Portanto, `$E0` é um perfil plausível, não prova do byte físico exato em todas as revisões. O valor `$AB` é o perfil de cartucho habilitado escolhido pelo emulador, não a execução do boot ROM.

A BIOS v1.3 documentada também grava o último valor escrito em `$3E` no primeiro byte da RAM (`$C000`). Na baseline sem BIOS, `work_ram` começa zerada e esse efeito não ocorre. Esse é mais um efeito pós-BIOS ausente no atalho de boot direto; não se deve copiar `$AB` para `$C000` sem uma política explícita ou execução real da BIOS.

### 3.2 PSG — o que é sustentado e o que continua em aberto

A documentação SMS Power sobre SN76489 relata que versões integradas Sega iniciam registradores de tom/ruído em zero e registradores de volume em um (silêncio); chips discretos podem apresentar valores iniciais aleatórios. Os registradores `tone_periods=[0;3]`, `noise_control=0` e `attenuations=[15;4]` do core atual são coerentes com a variante integrada Sega. Isso não determina a fase exata dos contadores internos ou o estado de cada bit do LFSR no instante de power-on.

Há uma divergência interna de especificação: o arquivo de especificação de áudio anexado descreve `tone_counters=[0;3]`, enquanto `Sn76489Core::reset_inner()` na main define `[1;3]`. A referência SMS Power consultada não resolve essa fase interna. Registrar como discrepância pendente; não alterar PSG nesta frente sem teste de hardware ou evidência temporal independente.

### 3.3 Mapper — limite importante da detecção atual

`detect_mapper` reconhece Codemasters por uma assinatura estreita no cabeçalho em `$7FE0`. Reconhece o perfil SegaExtended para ROMs acima de 1 MiB com cabeçalho TMR SEGA; **qualquer outro caso cai para MapperKind::Sega**.

Portanto, a enumeração atual não representa “mapper desconhecido”. O resultado Sega para uma ROM sem assinatura Codemasters não é prova independente do PCB físico. Para os dois jogos-alvo, a identificação precisa combinar hash local, cabeçalho, padrão de escritas e, quando possível, documentação do cartucho.

## 4. Z80 — reset não é power-on

O Z80 User Manual UM008011-0816 especifica que o sinal RESET limpa o Program Counter, I e R, desabilita o flip-flop de interrupção e seleciona IM 0. O texto não especifica um valor de reset para SP, registradores gerais, alternativos ou IX/IY.

A implementação de `Z80MicroSequencer::reset()` preserva os registradores gerais, alternativos, IX/IY e SP, e reinicia PC/I/R e o controle de interrupções. Isso é coerente com a distinção entre o pino RESET e o estado default de construção do emulador.

**Não alterar o Z80 para forçar SP=$DFF0** nesta investigação. Há valores diferentes em fontes de naturezas diferentes: SMS Official Docs recomenda SP=$DFF8 para compatibilidade; o caminho normal do SmsVdpTest executa `LD SP,$DFF0`; uma alegação de BIOS SP=$DFFF está marcada como não confirmada no documento de engenharia reversa do projeto; o Z80 RESET não define SP. Nenhum desses valores é prova de um valor universal de power-on. A stack deve ser inicializada pelo firmware/software quando necessário, e o perfil de boot deve preservar a incerteza em vez de inventar um estado físico.

### 2.4 Instrumentação de boot já existente

`SmsTemporalMachine::set_trace_enabled(true)` captura `BusCompleted`, `InternalCycleCompleted`, fases M1 e transições TH com timestamp, operação/endereço, T-states base, WAITs e dado. A baseline também expõe `SmsTemporalWorker::set_trace_enabled(true)` e `debug_snapshot()`: o worker drena `take_trace()` após cada frame ou comando para um coletor FIFO limitado (fila de ingresso 512; histórico 2.048), com contador de eventos descartados. É possível habilitar a captura antes de chamar `start()` para incluir o primeiro fetch, sem callback por instrução. O histórico é limitado e pode descartar eventos em boots longos; o snapshot atual inclui CPU, tempo, VDP status/raster, bancos/controle do mapper, memory-control e framebuffer, mas não expõe todos os registradores VDP nem o estado interno do PSG.

A PR #878 propõe aumentar a janela para 16.384 eventos, fila 4.096 e adicionar `arm_trace_on_next_start()`. É Draft e sua base está 73 commits atrás da main atual; não deve ser integrada diretamente. Usar como referência, não como dependência. O `port_write_trace` do barramento continua sendo apenas um buffer circular de 64 escritas de I/O e não substitui o trace de barramento.

O smoke campaign existente (`docs/testing/sms-in-game-rom-smoke-campaign.md`) carrega ROMs locais via `SmsTemporalWorker`, avança 120 frames e verifica instruções e frame não-preto. É útil como primeiro gate de regressão, mas não comprova jogabilidade, estado correto, áudio, nem primeira divergência. A campanha exige ROMs externas ao Git e região explícita.

PR #912 é um teste exploratório de HCounter/IRQ via Z80 e Scheduler com harness de ROM upstream. Os samples secundários de RunTest32/34 ainda divergem e permanecem ignorados; o PR é Draft e não altera hardware de produção. Reutilizar seu harness e evitar duplicar os testes. PR #913 documenta um problema separado de animação na tela de título de Road Rash: o jogo chega ao gameplay e foi jogável após o letreiro avançar, mas a animação inicial fica parada sem Pause. Assim, Road Rash continua sendo regressão positiva de boot/gameplay, porém não deve ser considerado PASS integral de temporização até essa questão separada ser resolvida.

## 5. SMS VDP Test v1.31 — interpretação correta dos valores

O código-fonte público do SmsVdpTest define em `ColdReset` um `JP DumpAllRegisters` de 10 ciclos. O caminho normal `WarmReset/main` só mais tarde executa `LD SP,$DFF0` e inicializa o VDP. Portanto, a tela de registradores não deve ser interpretada como uma fotografia abstrata dos valores universais do chip no instante físico t=0.

O README do teste descreve:
- VCounter como contador livre, legível em qualquer instante;
- HCounter como valor latched por transição TH 0→1;
- polling normal de HCounter retorna valor constante;
- a linha TH pode ser alterada por software via port $3F;
- a suíte depende de 228 ciclos Z80 por scanline.

O repositório público do teste não permitiu localizar, na fonte consultada, o corpo da rotina `DumpAllRegisters`. Assim, o instante exato das leituras e qualquer manipulação prévia de TH ainda precisam ser confirmados por disassembly da ROM binária exata. Não comparar VC/HC sem timestamp equivalente.

As diferenças Gearsystem/Ares registradas anteriormente permanecem observações de execução. A hipótese de que o Ares apresenta resíduos da BIOS é plausível, mas não foi comprovada apenas pelos valores mostrados.

### 5.1 O que os contadores realmente representam

O port $7E fornece o VCounter, que acompanha a linha de raster e varia conforme padrão de vídeo e modo. O port $7F fornece HCounter; o valor só é efetivo após o pulso de clock/TH correspondente. Portanto, VC/HC na tela do teste são amostras temporais, não registradores estáticos de power-on. A diferença de fase observada em Gearsystem/Ares pode ser consequência do tempo gasto antes da captura e da transição TH que latched o HCounter. Sem disassembly da rotina de dump e timestamp do read, não há comparação válida.

## 6. ROMs-alvo — identidade pública versus arquivo local

Os catálogos públicos No-Intro/libretro e MAME identificam:

| Jogo | Tamanho catalogado | CRC32 catalogado | SHA-1 catalogado |
|---|---:|---|---|
| Andre Agassi Tennis (Europe, Brazil) | 262.144 bytes | `F499034D` | `81DBACAD4739B98281C750D9AF21606275398FC8` |
| Ace of Aces (Europe) | 262.144 bytes | `887D9F6B` | `08A79905767B8E5AF8A9C9C232342E6C47588093` |

Esses são identificadores de DAT público, não hashes calculados dos arquivos locais do usuário. O MAME software list não fornece PCB explícito nessas duas entradas. Sem as ROMs locais não é possível confirmar igualdade byte a byte, calcular SHA-256 local, inspecionar o cabeçalho real, registrar a sequência de instruções ou comprovar o mapper do arquivo testado. As entradas MAME para os dois títulos não informam um campo PCB; portanto, o DAT confirma identidade do dump, mas não resolve por si só o mapper físico.

O runtime atual classifica como Sega qualquer ROM que não satisfaça a assinatura Codemasters (exceto o perfil SegaExtended). Essa regra será confrontada com os bytes e traces de cada ROM; não será aceita como prova do mapper físico.

## 7. Hipóteses e grau de evidência

| Hipótese | Evidência atual | Grau | Como refutar/confirmar |
|---|---|---|---|
| H1 — caminho temporal combina CPU default com mapa de memória pós-BIOS | CPU default PC/SP zero; sem BIOS; latch $AB | **Alta — fato estrutural** | Comparar trace com boot real de BIOS e primeiro estado do cartucho |
| H2 — CALL/PUSH antes de inicializar SP pode escrever em $FFFE/$FFFF e alterar bancos Sega | Teste sintético existente reproduz a colisão | **Alta para o mecanismo; baixa para os dois jogos** | Trace por ROM até primeira inicialização de SP e primeira escrita FFFC-FFFF |
| H3 — mapper real dos jogos pode divergir do mapper assumido | Detecção atual faz fallback Sega; DAT não traz PCB explícito | **Média como risco, sem confirmação por ROM** | Hash local, cabeçalho, dump de escritas e referência do PCB |
| H4 — diferenças do Registers Startup Values são efeito de BIOS/tempo de execução distinto | JP no vetor cold reset; SP é inicializado mais tarde; VC é livre e HC é latched por TH | **Média/alta para não equivalência de instante; baixa para atribuição a um emulador específico** | Disassembly da ROM e captura sincronizada em T-state conhecido |
| H5 — estado inicial de VDP/PSG/I/O pode não reproduzir estado deixado por BIOS | Defaults zerados/atenuados e I/O $FF; caminho temporal não executa BIOS | **Média como diferença entre perfis; causalidade dos black screens não provada** | Boot com BIOS real, trace de escritas da BIOS e comparação de estado no handoff |
| H6 — firmware catalogado ainda não é encaminhado ao worker temporal | UI persiste e escaneia firmware, mas informa explicitamente que boot ainda aguarda integração; `create_sms_worker` não passa BIOS | **Alta — limitação intencional e documentada** | Integrar bytes validados no host e testar caminho temporal completo antes de habilitar a opção |

Não há evidência que permita afirmar que Andre Agassi Tennis e Ace of Aces compartilham a mesma causa.

## 8. PRs e investigações relacionadas — evitar duplicação

- **PR #908** — BIOS convencional no runtime temporal: Draft, 17 commits exclusivos e 20 commits atrás da main; sem integração do chamador desktop e com handoff sintético. Não fazer merge direto.
- **PR #912** — teste exploratório de IRQ/HCounter com Z80/VDP/Scheduler e ROM upstream: Draft; amostras secundárias RunTest32/34 ainda divergem e estão ignoradas. Reutilizar o harness, sem duplicar nem alterar VDP por antecipação.
- **PR #913** — Road Rash: registro de travamento da animação de título, embora o jogo chegue ao gameplay e tenha sido jogável após avanço manual. É uma regressão temporal separada do boot; não invalida o uso do título como teste de boot positivo.
- **PR #881** — investigação de sprites em The Smurfs: Travel the World (Brazil) e Cosmic Spacehead. A ROM The Smurfs catalogada tem CRC32 `E14C8A1C`, SHA-1 `15b882ad3263cdc1e78c732d96f9bb18e25f4743` e SHA-256 `f3d493c5a673f78213f7544b3d9bacb1601721490d65e6bd041f4e01e5c6f85e`; ela inicia e chega ao gameplay, com defeito gráfico de ordem de sprites. Pode ser a mesma edição que o usuário chama de Schtroumpfs 2, mas só o hash local confirma; não misturar como regressão sem identidade.
- **PR #812** — catálogo de 17 ROMs homebrew/diagnóstico com hashes; nenhuma ROM foi executada, licença/proveniência permanece não verificada e os binários não foram incluídos. ZEXALL continua bloqueado até fechamento demonstrável de E7. Reutilizar apenas os metadados e respeitar o gate; não duplicar catálogo nem antecipar E8.
- **PR #878** — melhoria do trace: Draft e baseada em commit ancestral 73 commits atrás da main. A main já oferece captura bounded por `set_trace_enabled` e `debug_snapshot`; o PR propõe ampliar janela e `arm_trace_on_next_start`. Não integrar branch antiga diretamente.

## 9. PR #908 — avaliação inicial

PR #908 (`feat/sms-temporal-conventional-bios-8k`) adiciona `SmsTemporalMemoryOptions`, encaminha BIOS opcional para o barramento/máquina temporal, preserva bytes de BIOS nos caminhos de reset/stop/troca de ROM do worker e inclui testes sintéticos de fetch e handoff via `OUT ($3E)`.

Limitações observadas:
- PR em Draft;
- branch baseada em ancestral anterior à main auditada e divergente (17 commits exclusivos da PR e 20 commits da main desde a base reportada);
- não integra o chamador host/desktop que seleciona e fornece a BIOS ao novo construtor; a UI atual mantém a opção de boot desabilitada por padrão e informa explicitamente que o boot real ainda não está integrado;
- testes de handoff usam uma BIOS sintética, não os binários comerciais reais;
- restringe BIOS convencional ao perfil SMS1; SMS2 exige modelagem de arbitragem própria;
- checks principais de Rust/runtime, Scheduler e SMS1 VDP passaram no SHA da PR, mas vários jobs Windows/perfis estavam skipped.

Decisão: não fazer merge nem duplicar a implementação. A PR será usada como referência arquitetural; qualquer reaproveitamento exige portar/reconciliar com a main atual e completar integração/validação real.

## 10. Regressões e preservação

- Road Rash e Schtroumpfs 2 permanecem casos positivos obrigatórios.
- Não alterar Z80 compartilhado. O scorecard mantém E7 IN_PROGRESS e E8 BLOCKED; ZEXDOC/ZEXALL não fazem parte desta frente.
- Preservar estado serializado do Scheduler, transações suspensas, WAIT, memória, VDP, PSG e SRAM.
- Não mudar versões de Save State sem análise explícita de compatibilidade.

## 11. Plano de testes para B4–B6

1. **CPU:** construir e capturar o estado de power-on do sequenciador; testar RESET isolado em instruction boundary; validar PC/I/R/IFF/IM e preservação de SP/GP/IX/IY; registrar primeiro fetch e instruções iniciais.
2. **Memória:** testar BIOS ausente/presente, $3E=$AB/$E0, janelas $0000-$BFFF, RAM $C000-$FFFF, espelhamento e gating dos writes FFFC-FFFF pelo bit de cartucho.
3. **Mapper:** testes separados Sega/Codemasters; writes $FFFC-$FFFF; CALL/PUSH/POP/RET com SP em $0000 e em RAM; verificar mapper e RAM após cada write.
4. **VDP/I/O:** estado no tempo zero e após cada T-state; VCounter e HCounter com TH controlado; portas $3F/$7E/$7F/$BE/$BF; status e IRQ; comparar apenas no mesmo timestamp.
5. **PSG:** snapshot dos registradores e fases no tempo zero; sequência de writes; equivalência contínua/em blocos.
6. **Worker:** distinguir Pause, Reset, Stop, LoadRom e LoadState; verificar SRAM, BIOS, ROM identity, região, versão e Scheduler.
7. **ROMs:** CRC32/SHA-1/SHA-256 local, mapper efetivo, primeiros 256/4096 T-states, todas as escritas de memória/I/O, primeiro CALL/PUSH e primeira divergência.
8. **Regressões:** Andre Agassi Tennis, Ace of Aces, Road Rash e Schtroumpfs 2; executar em baseline e perfis de boot comparáveis.
9. **Determinismo:** contínuo versus blocos, WAIT, suspensão/retomada e Save State/restore.

## 12. Arquivos candidatos a alteração — ainda não alterados

- `native/sms_core/src/memory.rs`: contrato de BIOS/latch e mapper; apenas após fechar evidência.
- `native/sms_core/src/temporal_bus.rs`: opções de memória/BIOS no caminho temporal.
- `native/sms_core/src/temporal_machine.rs`: construtores explícitos de perfil de boot; sem tocar na semântica do Z80.
- `rust/runtime/sms-runtime/src/worker.rs`: lifecycle de BIOS/ROM/reset/save-state e integração coarse-grained.
- chamador host/desktop efetivo: encaminhar firmware já validado, sem filesystem no Scheduler.
- testes em `native/sms_core/tests` e testes de runtime correspondentes.
- documentação em `docs/audits` e matriz de conformidade SMS.

Nenhum desses arquivos de runtime foi alterado nesta etapa. O próximo passo é fechar a inspeção dos chamadores host, localizar o código exato da rotina de dump do SmsVdpTest e obter os binários locais para reprodução/hash antes de escolher a correção.


## 13. Fontes consultadas e natureza da evidência

### Documentação de hardware/firmware

- Zilog, *Z80 CPU User Manual*, UM008011-0816 — RESET, duração mínima e registradores afetados: https://www.zilog.com/docs/z80/um0080.pdf
- SMS Power, *BIOSes* — rotina de seleção de slots e comportamento por revisão: https://www.smspower.org/Development/BIOSes
- SMS Power, *SMS Official Docs* — fluxo de boot de BIOS, RAM/stack e mapa de hardware: https://www.smspower.org/Development/SMSOfficialDocs
- SMS Power, *Port $3E* — bits de habilitação e semântica ativa baixa: https://www.smspower.org/Development/Port3E
- SMS Power, *Memory Map* — RAM espelhada, slots e registradores Sega: https://www.smspower.org/Development/MemoryMap
- SMS Power, *Cartridge Mappers* — Sega/Codemasters e protocolos de mapper: https://www.smspower.org/Development/CartridgeMappers
- SMS Power, *SN76489* — estado de registradores em variantes integradas/discretas: https://www.smspower.org/Development/SN76489
- SMS Power, *HCounter* e *ScanlineCounter* — leitura latched e contadores dependentes de raster: https://www.smspower.org/Development/HCounter ; https://www.smspower.org/Development/ScanlineCounter
- SmsVdpTest v1.31, fonte pública: https://github.com/FluBBaOfWard/SmsVdpTest/blob/main/testmain.asm

### Identidade de ROM — catálogos, não arquivos locais

- No-Intro/libretro, DAT Sega Master System: https://github.com/libretro/libretro-database/blob/master/metadat/no-intro/Sega%20-%20Master%20System%20-%20Mark%20III.dat
- MAME software list, SMS XML: https://sources.debian.org/src/mame/0.251%2Bdfsg.1-1/hash/sms.xml

### Documentação interna consultada

- `docs/audits/sms-bios-boot-sequence-reverse-engineering.md`
- `docs/audits/sms-cartridge-boot-integrity-execution-plan.md`
- `docs/audits/andre-agassi-tennis-sms-black-screen-investigation-2026-10-29.md`
- `docs/audits/sms-ace-of-aces-region-boot-investigation-2026-10-29.md`
- `docs/testing/sms-in-game-rom-smoke-campaign.md`
- `docs/architecture/sms-firmware-manager-ui-and-region-policy-2026-11-15.md`
- `docs/architecture/sms-built-in-game-firmware-catalog-2026-11-15.md`
- `docs/architecture/shared-debugger-mvp.md`
- `docs/architecture/sms-boot-contract.md`
- `docs/audits/sms-boot-conformance-matrix-2026-11-15.md`

Documentação técnica descreve comportamento geral; DAT identifica dumps; código confirma a policy efetivamente implementada; testes sintéticos provam somente seus cenários. Nenhuma dessas classes, isoladamente, prova a execução dos dois jogos no arquivo local.
