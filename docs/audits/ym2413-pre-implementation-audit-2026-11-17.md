# YM2413 — auditoria pré-implementação no main

- **Data:** 2026-11-17
- **Baseline remoto:** `main` = `b82e0df4c9840b2cd59f090278b907cbb321869f`
- **Estado:** M0 concluída — auditoria documental e arquitetural; nenhum código de chip foi implementado.
- **Escopo:** YM2413 integrado do perfil Japanese Master System; não inclui o FM Sound Unit externo do Mark III.

## 1. Gate de entrada — PR #992

A Fase 2 foi iniciada somente após confirmação remota da integração da PR #992:

- PR #992: **MERGED**.
- HEAD da branch da PR: `f0ad78d9617c14e8c06754d5daeb08d6e5e60ae6`.
- Merge commit e HEAD de `main`: `b82e0df4c9840b2cd59f090278b907cbb321869f`.
- Workflow pós-merge: [run 44186646679](https://github.com/leonardo201800478/sega-emulator/actions/runs/44186646679), concluído.
- Gates aplicáveis ao diff: Change impact classification, Rust workspace regression, Documentation patch hygiene, Windows PowerShell tooling syntax, Windows SDL3 input validation e Windows x64 Slint/WGPU/Vulkan POC: **PASS**.
- Jobs não aplicáveis ao diff e explicitamente **SKIPPED**: SMS1 VDP, CPAL output, PSG core, workflow validation, Scheduler stress, input core, SMS frame-pacing ROM build, geração manual de lockfiles e Z80 focused validation.

Nenhum log de falha foi lido porque não havia falha aplicável. Os jobs SKIPPED não são evidência de validação física de áudio, PSG, Scheduler, VDP ou Z80. A integração da PR #992 satisfaz o gate de entrada da Fase 2, mas não valida o YM2413 nem o caminho físico de áudio.

## 2. Resultado executivo

O repositório já possui uma especificação M0/M1 do YM2413 em `docs/architecture/ym2413-fm-model.md`, mas **não possui implementação YM2413 no runtime**. O caminho correto é adicionar o chip à máquina SMS existente e ampliar o fluxo de áudio atual, sem criar uma segunda máquina, CPU, Scheduler, PSG, worker, mixer ou pipeline.

Há cinco lacunas de integração que precisam ser tratadas explicitamente:

1. `SmsTemporalBus` não decodifica $F0/$F1/$F2; as escritas caem no I/O genérico e não chegam a um chip FM.
2. O perfil SMS não representa explicitamente a presença do YM2413 integrado; território e revisão de VDP não bastam para identificar uma placa Japanese SMS.
3. O `AudioMixer` já soma múltiplas fontes, mas `AudioSession`/`NativeAudioSession` expõem apenas um bloco mono por chamada.
4. O PSG entrega PCM na taxa interna configurada de 48 kHz; o YM2413 tem um período de saída derivado de 72 clocks de entrada, aproximadamente 49,716 kHz. As fontes precisam ser alinhadas antes da soma.
5. O barramento e o estado da máquina ainda não guardam latch, pipeline, clock residual, saídas MO/RO ou estado do YM2413. O readback de $F2 também depende de C-Sync, sinal que não foi localizado no runtime atual.

A decisão de reutilizar o domínio de clock do PSG permanece **PENDENTE**: a frequência nominal coincide nos modelos consultados, mas isso não prova por si só a conexão elétrica nem o alinhamento de fase da placa japonesa.

## 3. Auditoria do código ativo no SHA de baseline

### 3.1 Scheduler e linha do tempo

Arquivos auditados:

- `native/sms_core/src/temporal.rs`
- `native/sms_core/src/temporal_bus.rs`
- `native/sms_core/src/temporal_machine.rs`
- `native/machine_scheduler/src/lib.rs`
- `rust/runtime/sms-runtime/src/worker.rs`

Estado observado:

- `SmsTemporalCoordinatorState` está na versão 5.
- Os domínios registrados são Z80 (ID 0), PSG (ID 1) e VDP (ID 2).
- O restore do coordenador exige exatamente esses três domínios e valida suas razões/tempos.
- O worker SMS ativo usa `SMS_TEMPORAL_MASTER_TICKS_PER_TSTATE = 1`: o tick lógico usado por essa máquina corresponde a um T-state Z80, não a um tick de 10,738635 MHz.
- Nesse perfil, o domínio PSG avança um ciclo por tick lógico. O core SN76489 é configurado com 3.579.545 Hz e PCM de 48.000 Hz.
- `SmsTemporalBusState` está na versão 20; `SmsTemporalMachineState` está na versão 33.

Consequência: não se deve interpretar a razão 1/1 do domínio PSG como prova de fase elétrica comum entre PSG e YM2413. O manual Yamaha especifica o oscilador de 3,579545 MHz no XIN/XOUT do YM2413. O MAME configura o YM2413 do SMS japonês em XTAL(10,738,635)/3, igual à frequência nominal do Z80 NTSC; isso é uma referência secundária de modelagem, não prova de que todas as revisões físicas compartilhem a mesma fase.

**Decisão provisória:** manter o Scheduler único. Só reutilizar o `ClockAdvance` do domínio PSG depois de confirmar a fonte/relacionamento de clock da placa alvo. Se a evidência indicar oscilador independente ou fase diferente, registrar um domínio explícito no mesmo `MachineScheduler`, versionando e validando o snapshot do coordenador. Não criar Scheduler paralelo.

### 3.2 Barramento SMS e portas

Arquivos auditados:

- `native/sms_core/src/temporal_bus.rs`
- `native/sms_core/src/io.rs`

Estado observado:

- `SmsTemporalBusState` contém memória, I/O, PSG, VDP, trace de escritas e estado do coordenador; não contém FM.
- `read_port` trata VDP e, no caminho genérico, delega a `SmsIo`.
- `SmsIo::read_port` trata os ports de controle/input existentes; $F2 não tem implementação de readback e atualmente cai no valor genérico $FF.
- `write_port` trata VDP, PSG, memory control e I/O. $F0/$F1/$F2 chegam ao caminho genérico de `SmsIo`, que não possui latch nem registradores FM.
- Não há estado de latch de endereço YM2413, deadline de escrita ou contador C-Sync no barramento atual.

A evidência de hardware para Japanese SMS diferencia o mapa de I/O: $F0 escreve endereço, $F1 escreve dados e $F2 é leitura/escrita de controle. O MAME também modela esses três ports sem espelhamento. O fórum técnico do SMS Power registra medições de que, no Japanese SMS, $3E e $3F são mapeados somente nos endereços exatos; escritas em $00-$3D não têm efeito. O decodificador atual do barramento aceita espelhos de $3E/$3F por paridade em todo o intervalo $00-$3F. Portanto, a integração FM exige um perfil de decodificação específico para Japanese SMS, sem alterar silenciosamente o comportamento dos perfis SMS existentes.

#### Contrato de $F2 no Japanese SMS

As medições publicadas para o Japanese SMS indicam:

| Bits | Leitura de $F2 |
|---|---|
| D7 | contador C-Sync bit 11 |
| D6 | contador C-Sync bit 7 |
| D5 | contador C-Sync bit 3 |
| D4–D2 | zero |
| D1–D0 | últimos bits escritos |

Na escrita, os dois bits inferiores controlam o roteamento/mute:

| Valor | SN76489 | YM2413 |
|---:|---|---|
| $00 | audível | silenciado |
| $01 | silenciado | audível |
| $02 | silenciado | silenciado |
| $03 | audível | audível |

O bit 0 habilita a saída FM; o bit 1 controla o mute do PSG. O estado de síntese não deve ser parado por um mute sem evidência: o chip continua avançando e apenas a saída é roteada/silenciada.

O comportamento do Mark III com FM Sound Unit externo é diferente e fica fora do escopo desta implementação. Não transportar sua seleção mutuamente exclusiva para o Japanese SMS.

**Bloqueio específico:** o contador de $F2 é incrementado por pulsos de C-Sync que chegam ao 315-5297. Não foi encontrado no código atual um evento/sinal C-Sync exportado pelo VDP para esse caminho. Não derivar o contador de scanlines ou do relógio da GUI por aproximação. A origem e a cadência do C-Sync precisam ser expostas por uma interface de hardware verificável antes de declarar o readback completo.

### 3.3 PSG, mixer, resampler e sessão de áudio

Arquivos auditados:

- `native/sn76489/src/psg.rs`
- `native/audio_runtime/src/psg.rs`
- `native/audio_runtime/src/dsp.rs`
- `native/audio_runtime/src/session.rs`
- `native/audio_runtime/src/lib.rs`
- `rust/runtime/sms-runtime/src/worker.rs`
- `native/audio_output/src/lib.rs`

Estado observado:

- `DEFAULT_PCM_SOURCE_RATE_HZ` é 48.000 Hz.
- O worker SMS configura o PSG com clock de 3.579.545 Hz e taxa PCM interna de 48 kHz.
- `AudioMixer::mix_mono_into` já aceita várias fontes mono, desde que tenham o mesmo número de frames, e soma cada fonte com ganho próprio.
- `AudioSession::process_samples` aceita somente um slice mono por chamada.
- `NativeAudioSession::process_samples_with_policy` constrói atualmente uma única `MixSource`; portanto, chamar a sessão separadamente para PSG e FM concatenaria blocos no tempo, não os misturaria.
- `PcmResampler::new(input_rate, output_rate, ...)` recebe taxas explícitas. O caminho atual não está fixado em 44,1 kHz: a fonte padrão SMS é 48 kHz e a saída é negociada com o host.
- O worker drena um único bloco PSG por frame. O modo H2 é um renderer alternativo da fonte PSG, não um segundo PSG nem um mixer FM.
- O processamento analógico opcional e o resampler final vivem na sessão nativa; a GUI não controla clocks de chip.

A taxa de 49,716 kHz do YM2413 é derivada de 3.579.545 / 72, não uma taxa final de dispositivo. A saída FM deve ser convertida de forma determinística para uma base comum de 48 kHz antes do mixer multi-source atual, ou o mixer/sessão deve ser generalizado para receber fontes com taxas e timestamps distintos. O estado/fase do conversor por fonte precisa sobreviver a blocos e a restore.

### 3.4 Desktop, preferências e seleção de máquina

Arquivos auditados:

- `native/sega_desktop/src/main.rs`
- `native/sega_desktop/src/desktop_session.rs`
- `rust/runtime/sms-runtime/src/worker.rs`

Estado observado:

- O desktop ativo usa Slint e possui uma única sessão/worker por máquina selecionada.
- `DesktopSystem` distingue SMS1, SMS2 e SG-1000, mas não distingue explicitamente uma placa Japanese SMS com YM2413 integrado.
- A construção do worker SMS recebe perfil/revisão de VDP e território; não recebe um perfil físico explícito de áudio/FM.
- As preferências persistidas já guardam renderer PSG/H2 e configurações DSP/HPF/LPF, mas não há `FmMode` nem catálogo de compatibilidade YM2413.
- Não foi encontrado catálogo FM por título/revisão/hash no código ativo.

Consequência: território não pode ser usado como sinônimo de presença física do chip. O perfil de placa/áudio deve ser independente da região do software. A GUI apenas persiste e apresenta Auto/On/Off; a decisão de disponibilidade e a aplicação de comandos pertencem ao worker/máquina existente.

### 3.5 Save state e continuidade

O estado atual do barramento não inclui YM2413. A integração terá de serializar pelo menos:

- registradores e latch de endereço;
- timestamp e deadline das escritas;
- contador de clock, slot e subfase;
- PG/EG/LFO/feedback e estado de ruído/rhythm;
- estado de saída MO/RO e amostras pendentes;
- política/perfil físico e controle $F2;
- conversor de taxa FM→48 kHz e seu histórico, se fizer parte do estado determinístico da máquina.

A sessão de áudio host atual não faz parte do `SmsTemporalMachineState`. O projeto deve definir claramente se o restore reinicia a sessão de saída ou se o estado de DSP/resampling também será serializado. Não declarar continuidade de áudio bit-exata se o histórico do resampler/mixer for descartado.

## 4. Evidência técnica — fatos, medições, inferências e lacunas

### 4.1 Documentado no manual Yamaha

- Entrada do chip por XIN/XOUT com oscilador de referência de 3,579545 MHz; faixa de operação indicada de 2–4 MHz.
- 9 canais melódicos, dois operadores lógicos por canal e uma célula de operador reutilizada em 18 passagens seriais.
- Modo alternativo com 6 canais melódicos e 5 vozes de rhythm.
- 15 instrumentos melódicos predefinidos, 5 vozes de rhythm e um conjunto de registradores para instrumento do usuário.
- Mapa funcional: $00–$07 user tone, $0E rhythm, $0F test, $10–$18 F-number LSB, $20–$28 F-number MSB/BLOCK/KEY/SUS, $30–$38 instrument/volume.
- Escrita de endereço: aguardar 12 ciclos do clock mestre antes da escrita de dados.
- Escrita de dados: aguardar 84 ciclos do clock mestre antes da próxima escrita de endereço ou dados. O manual diz que dados ficam incertos se o intervalo não for respeitado.
- DAC de 9 bits e saídas distintas MO (melody) e RO (rhythm). Ambas são source followers e exigem circuito integrado/estágio posterior, amplificação e, para melhor qualidade, filtragem.

A especificação de 4 clocks por slot aparece na transcrição japonesa do manual de aplicação e é corroborada pela engenharia reversa; a taxa de 72 clocks por período completo é derivada de 18 × 4. A taxa resultante, aproximadamente 49.716,04 Hz, é uma taxa interna de atualização por canal, não a taxa final do host.

### 4.2 Medido / engenharia reversa

- A observação publicada no MSX Resource Center descreve saída MO/RO multiplexada: rotação de 72 clocks, valor de cada canal ativo por cerca de 3 clocks e retorno ao nível neutro no restante do intervalo.
- As notas de Andete registram a rotação de 72 clocks e investigam o pipeline/shift registers, latência de escrita, PG, EG, feedback e estado inicial.
- Charles MacDonald publicou medições do $F2 no Japanese SMS: bits do contador C-Sync, bits de mute e ausência de espelhos de $3E/$3F.
- O esquema de Japanese SMS publicado no SMS Power foi traçado seguindo trilhas da placa; o autor declara que pode não ser 100% preciso. É evidência secundária útil, não um esquema oficial certificado.

### 4.3 Inferido / corroborado, ainda não suficiente para congelar

- O MAME configura CPU e YM2413 do SMS japonês a partir de XTAL(10,738,635)/3, o que corrobora frequência nominal de 3.579.545 Hz.
- Essa configuração não prova, isoladamente, a mesma fonte elétrica, fase inicial ou ausência de deriva entre a CPU/PSG e o YM2413 em todas as revisões de placa.
- A integração do áudio SMS deve preservar MO e RO separadamente até que o circuito de soma/ganho da revisão alvo esteja confirmado. Não copiar o circuito MSX/SFG-05 para o SMS.

### 4.4 Lacunas que permanecem abertas

| ID | Lacuna | Tratamento |
|---|---|---|
| FM-A01 | Fonte e fase elétrica do clock YM2413 na placa Japanese SMS alvo | Confirmar em esquema confiável ou medição; decidir domínio Scheduler depois |
| FM-A02 | Rede analógica SMS de MO/RO, ganho relativo, carga e filtragem | Não normalizar/somar pinos sem modelo da placa |
| FM-A03 | C-Sync do VDP até o 315-5297 e contador de $F2 | Expor evento de hardware verificável; não derivar por GUI/scanline presumida |
| FM-A04 | Perfil exato de placa/revisão com FM integrado | Criar perfil explícito, separado de Territory e VDP revision |
| FM-A05 | Catálogo de compatibilidade por título/revisão | Exigir identidade/hash de ROM e evidência de writes FM; sem correspondência exata, Auto permanece desativado |
| FM-A06 | Comportamento de writes abaixo de 12/84 ciclos | O manual declara dados incertos; não inventar aceitação determinística |
| FM-A07 | Continuidade do conversor FM→48 kHz e do mixer em save state | Definir ownership e versão do snapshot antes de prometer continuidade de áudio |
| FM-A08 | Correspondência dos presets internos do YM2413 original | Não importar ROM VRC7/YMF281B/FHB013 como YM2413 sem validação |

## 5. Contrato arquitetural proposto

1. O YM2413 será um core digital puro em Rust, sem dependência de SMS, GUI, CPAL, SDL3 ou WGPU.
2. O core expõe reset, write-address, write-data, avanço explícito de clocks, saída MO/RO e snapshot/restore.
3. A máquina SMS existente será proprietária do adaptador temporal e do estado de barramento FM. Não criar segunda CPU, Scheduler, PSG, worker ou runtime.
4. O adaptador FM só consumirá o `ClockAdvance` do PSG depois que FM-A01 for fechado. Caso contrário, usar domínio explícito no mesmo Scheduler.
5. $F0/$F1/$F2 serão decodificados somente no perfil de hardware que representa o Japanese SMS integrado. O Mark III FM Sound Unit externo permanece fora do escopo.
6. O perfil de hardware/FM será independente do território do software. O decodificador de I/O japonês também precisa respeitar a ausência de espelhos de $3E/$3F.
7. $F2 controla roteamento/mute, não o avanço interno dos chips. SN76489 e YM2413 devem continuar clockando e podem estar audíveis simultaneamente no Japanese SMS.
8. A sessão de áudio receberá PSG e FM alinhados em blocos de 48 kHz e chamará o mixer uma vez por intervalo. O resampler final para a taxa CPAL negociada continua único.
9. O caminho FM preservará MO e RO e modelará a conversão/integração antes da soma mono; os coeficientes do SMS ficam pendentes de evidência da placa.
10. Nenhum jogo receberá dados FM artificiais. Auto exige correspondência exata no catálogo; On é override explícito de disponibilidade, mas só produz áudio quando o software realmente escreve dados nos registradores; Off desabilita a fonte FM.
11. Save state/replay incluirá o estado do chip e do adaptador; a política para estado de resampling/mixer será definida antes do aceite de continuidade.
12. Não implementar o módulo externo FM do Mark III nem ativar FM em SG-1000.

## 6. Plano incremental

| Etapa | Entrega | Gate principal |
|---|---|---|
| M0 — esta PR | Auditoria e reconciliação documental; sem código de hardware | CI documental PASS |
| M1 | Fechar perfil de placa, clock/fase, contrato $F0–$F2, C-Sync, MO/RO e identidade de ROM | Evidências classificadas; decisões sem hipótese promovida a fato |
| M2 | Core puro Rust: reset, latch, registradores, clock, slots, deadlines de escrita e estado | Testes determinísticos de registradores, clock, 12/84 e invariância de blocos |
| M3 | PG, EG, AM/PM, feedback, presets e rhythm | Vetores dourados e diferencial contra referências identificadas |
| M4 | Adaptador temporal na máquina SMS e integração no barramento | Writes no timestamp de conclusão; Scheduler único; snapshot/restore |
| M5 | $F0/$F1/$F2, perfil Japanese SMS e C-Sync | Testes de decode, readback, mute e coexistência PSG+FM |
| M6 | Saídas MO/RO, conversão FM→48 kHz e mixer multi-source | Alinhamento contínuo/em blocos, headroom e resampling determinístico |
| M7 | Preferências Auto/On/Off e catálogo por título/revisão/hash | Auto conservador; override sem fabricar dados FM |
| M8 | Save state/replay e regressões SMS1/SMS2/SG-1000 | Restore/replay equivalentes e regressões compartilhadas PASS |

O primeiro código de produção deve começar pelo core puro, não pela GUI ou pelo barramento. As incertezas específicas da placa podem permanecer isoladas no adaptador até serem fechadas; não devem contaminar o core.

## 7. Matriz inicial de validação

Todos os itens abaixo permanecem **NOT RUN** nesta baseline; não foram executados testes de YM2413 porque ainda não existe implementação.

- Registro/latch e mapa $00–$38: NOT RUN.
- 12 ciclos após endereço e 84 após dados: NOT RUN.
- 18 slots / 4 clocks por slot / 72 clocks por período: NOT RUN.
- MO/RO, janela ativa de 3 clocks e nível neutro: NOT RUN.
- PG/MUL/FNUM/BLOCK, EG, AM/PM e feedback: NOT RUN.
- Presets originais e user patch: NOT RUN.
- Rhythm mode: NOT RUN.
- $F0/$F1/$F2 e readback C-Sync: BLOCKED pela ausência do sinal C-Sync no runtime atual.
- Clock/fase da placa Japanese SMS: BLOCKED até evidência de placa.
- Mistura PSG+FM e conversão para 48 kHz: NOT RUN.
- Auto/On/Off e catálogo por revisão/hash: NOT RUN.
- Snapshot/restore/replay FM: NOT RUN.
- Regressão SMS1/SMS2 e SG-1000: NOT RUN.
- Testes físicos de placa/saída analógica: NOT RUN.

## 8. Referências e proveniência

### Manual e hardware

- [Yamaha YM2413 Application Manual — cópia digitalizada do manual Yamaha](https://www.waveguide.se/?article=46&file=ym2413am.pdf)
- [Yamaha YM2413 Application Manual — espelho SMS Power](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual)
- [Transcrição japonesa do manual de aplicação](https://d4.princess.ne.jp/msx/datas/OPLL/YM2413AP.html)
- [SMS Power — Audio Control Port](https://www.smspower.org/Development/AudioControlPort)
- [SMS Power — medições e esquemas SG-1000 II / Mark III / Japanese SMS](https://www.smspower.org/forums/15878-SG1000IISEGAMARKIIIAndJAPSMSSchematic)

### Engenharia reversa

- [MSX Resource Center — YM2413 reverse engineering progress](https://www.msx.org/photoshoots/ym2413-reverse-engineering-progress)
- [SMS Power — reverse engineering notes, 2015-12-16](https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-12-16)
- [Andete — repositório de notas de reverse engineering](https://github.com/andete/ym2413)

### Referências de implementação — não são autoridade única

- [emu2413 (MIT, implementação em C)](https://github.com/digital-sound-antiques/emu2413) — referência diferencial; não integrar código C ao runtime Rust.
- [ymfm (BSD-3-Clause, inclui YM2413)](https://github.com/aaronsgiles/ymfm) — referência secundária; o próprio projeto declara que não busca precisão digital de 100%.
- [Nuked-OPLL (GPL-2.0, cycle-accurate)](https://github.com/nukeykt/Nuked-OPLL) — referência de comparação; não copiar código/tabelas para o projeto sem decisão explícita de licença.
- [MAME SMS driver](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp) — corroborador de mapeamento e clock; não substitui evidência física.
- [MAME SMS machine implementation](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms_m.cpp) — corroborador de $F2 e do contador C-Sync.

Nenhuma implementação de referência foi copiada. As referências C/C++ são apenas fontes de comparação; o runtime final permanece 100% Rust. O repositório de notas de Andete foi consultado como evidência técnica, não como fonte de código reutilizável.

## 9. Limite desta entrega

Esta auditoria não declara o YM2413 implementado, audivelmente correto, cycle-accurate, validado em hardware ou compatível com qualquer ROM específica. Não houve mudança de código, testes de chip ou teste físico. A próxima etapa será uma PR incremental separada para o core Rust e seus vetores, sem merge automático.
