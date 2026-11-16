# SMS Boot Contract — proposta arquitetural

**Data:** 2026-11-15  
**Status:** proposta para revisão; não implementada.  
**Baseline de referência:** `90036d8d9f988640faf1540d2b4dc1548780c13f`  
**Branch:** `fix/sms-boot-compatibility`

## 1. Princípio

Boot não é um único conjunto de valores de registradores. É uma sequência causal que depende do modelo de console, presença/revisão da BIOS, mapper, região e estado anterior.

O contrato deve separar:

1. **Power-on físico:** estado inicial de cada dispositivo para um modelo/revisão; valores não documentados permanecem explicitamente não determinados.
2. **Boot direto de cartucho:** caminho de compatibilidade sem executar BIOS. Não é sinônimo de estado pós-BIOS.
3. **Boot por BIOS:** CPU inicia em $0000 e executa bytes reais da BIOS pelo MachineScheduler; mapeamento e periféricos evoluem por operações normais de barramento.
4. **Handoff da BIOS:** ponto observado no fluxo da BIOS em que o cartucho passa a responder e a execução é transferida; não é um template universal de registradores.
5. **Reset de console:** operação elétrica/temporal a especificar por modelo; não deve ser representada automaticamente por reconstrução da máquina.
6. **Reset solicitado pela aplicação:** política de produto explicitamente definida; na baseline atual é reconstrução integral da máquina, deixando-a pausada.
7. **Restore de Save State:** restaura o estado completo e não reexecuta BIOS nem aplica defaults de power-on.

Nenhuma regra deve ser condicionada ao nome de um jogo.

## 2. Tipos conceituais propostos

Os nomes abaixo descrevem o contrato, não exigem que os tipos Rust tenham exatamente esta sintaxe.

```rust
enum SmsBootPath {
    DirectCartridge(DirectBootPolicy),
    Bios(BiosBootIdentity),
}

enum SmsResetOperation {
    ColdPowerOn,
    ConsoleReset,
    ApplicationColdReset,
    RestoreSaveState,
}

enum Evidence<T> {
    HardwareDocumented { value: T, source: &'static str },
    BiosDerived { value: T, bios_sha256: [u8; 32], capture_point: String },
    EmulatorPolicy { value: T, policy_version: u32, rationale: String },
    Unknown,
}

enum MapperDetection {
    Known { mapper: MapperKind, evidence: MapperEvidence },
    Unknown,
    Ambiguous { candidates: Vec<MapperKind> },
}
```

O estado de cada subsistema deve carregar proveniência suficiente para que um valor de policy não seja apresentado como fato físico. A enumeração `Evidence` é conceitual: não se deve adicionar metadados pesados ao caminho de cada T-state.

## 3. Perfil de hardware

Um `SmsBootProfile` deverá identificar, no mínimo:

- revisão/modelo do console (SMS1, SMS2 e variantes efetivamente suportadas);
- revisão/ASIC do VDP;
- região e padrão de vídeo;
- caminho de boot (direto ou BIOS);
- identidade criptográfica e revisão da BIOS, quando aplicável;
- mapper selecionado e evidência da detecção;
- política de RAM e SRAM;
- política de estado inicial de CPU, VDP, PSG e I/O;
- origem/proveniência de cada valor não determinado pelo hardware.

O perfil é selecionado pelo host antes de criar a máquina. O host fornece bytes e configuração; o runtime executa o hardware. A GUI não calcula ciclos nem implementa dispositivos.

## 4. Contratos por caminho

### 4.1 DirectCartridge — sem BIOS

- Cartucho é o dispositivo visível no vetor $0000.
- PC inicial = $0000 como política de entrada do cartucho.
- Memory Control deve habilitar o slot do cartucho e a RAM conforme o modelo.
- O perfil atual usa $AB; esse byte é uma escolha de boot direto/pós-BIOS do emulador, não evidência de que a BIOS foi executada.
- RAM, VDP e I/O devem usar valores de power-on documentados quando existirem; caso contrário, um valor determinístico do emulador deve ser explicitamente classificado como policy.
- SP, AF/BC/DE/HL, alternativos e IX/IY não podem ser declarados valores universais do Z80 após RESET. Não forçar SP=$DFF0/$DFF8/$DFFF sem perfil sustentado.
- O boot direto não pode copiar silenciosamente efeitos da BIOS para RAM, VDP ou PSG. Em particular, não gravar artificialmente o latch $3E em $C000 para simular uma BIOS que não executou.

O perfil direto é útil para desenvolvimento e para emuladores sem BIOS, mas deve declarar suas limitações de fidelidade.

### 4.2 Bios — execução real

- Validar a imagem por tamanho, SHA-256, revisão e modelo suportado.
- Inicializar o mapa para que a BIOS real esteja visível no vetor $0000.
- Inicializar CPU conforme o sinal RESET documentado; os demais registradores não especificados usam uma policy determinística explícita.
- O VDP, RAM, PSG e I/O devem iniciar conforme o modelo físico; a BIOS deve executar suas próprias escritas de inicialização.
- Não pré-aplicar efeitos que a BIOS executará. Isso duplicaria side effects e quebraria ordem causal.
- A BIOS deve ser executada pela CPU compartilhada, barramento temporal e MachineScheduler. Não executar rotinas BIOS atomicamente na GUI ou no host.
- O suporte da PR #908 limita-se a BIOS convencional de 8 KiB no perfil SMS1. Não declarar SMS2 compatível sem modelar e validar a arbitragem correspondente.

A documentação SMS Power descreve, em alto nível, execução inicial da BIOS em $0000, cópia de uma rotina para RAM em $C700, sondagem de slots e transferência para o cartucho. A sequência byte a byte deve vir do disassembly da BIOS identificada.

### 4.3 Handoff da BIOS

O handoff é definido por evidência de execução, não por um valor de registrador presumido:

1. a BIOS termina a rotina de seleção;
2. o latch $3E é escrito pela CPU na transação de I/O correspondente;
3. o mapa passa a expor o cartucho;
4. a BIOS transfere o fluxo para o vetor do cartucho;
5. o primeiro fetch do cartucho é observado no Scheduler.

O snapshot de handoff deve registrar timestamp, PC, SP, AF/BC/DE/HL, IX/IY, alternativos, I/R, IFF/IM, memory-control, RAM relevante (incluindo $C000), mapper/bancos, estado do VDP, PSG e I/O. O snapshot é específico da imagem de BIOS e do modelo.

A BIOS v1.3 não inicializa os registradores de paginação antes da sondagem; o estado inicial do mapper deve, portanto, expor os dados esperados ao cabeçalho. Para o mapper Sega, o perfil atual [0,1,2] satisfaz o caso comum de slot 1/bank 1, mas a seleção deve ser sustentada pelo mapper efetivo.

### 4.4 Reset

**RESET do Z80:** o manual Zilog especifica RESET ativo por pelo menos três ciclos completos de clock; reinicia PC, I, R e controle de interrupções/IM. Não define SP, registradores gerais ou IX/IY. O helper atual `SmsTemporalMachine::reset_cpu()` é atômico e não consome três T-states; não deve ser apresentado como pino RESET temporal.

**Reset de console:** exige contrato próprio para cada modelo, incluindo quais periféricos recebem reset, quais memórias/latches são preservados e quando o Scheduler retoma. Não está implementado como sequência elétrica temporal independente na baseline.

**Reset da aplicação:** na baseline, `WorkerCommand::Reset` reconstrói a máquina inteira, preserva SRAM e deixa o worker pausado. Deve ser rotulado como reset frio da aplicação até que um reset de console real seja implementado.

**Stop/power-off:** na baseline, `WorkerCommand::Stop` também reconstrói a máquina, preserva ROM/SRAM e a mantém parada. Start posterior inicia essa máquina recém-construída. Stop e Reset compartilham o rebuild, diferindo principalmente no lifecycle/estado running; isso não representa dois sinais elétricos distintos.

### 4.5 LoadRom

- A substituição de ROM cria uma nova máquina e inicia novo epoch temporal.
- A baseline não encaminha BIOS ao worker; o novo worker é construído em boot direto.
- A política de firmware futura deve preservar a BIOS selecionada ao trocar cartucho, se esse for o comportamento de produto.
- A política atual rejeita `load_rom` quando há SRAM no cartucho; persistência/exportação é responsabilidade do host.
- Não manter estado de CPU/VDP/PSG do cartucho anterior.

### 4.6 Save State / Restore

- Restore não é reset nem boot.
- Deve restaurar CPU e microsequenciador, barramento, memória, mapper, BIOS image identity, VDP, PSG, I/O, Scheduler, clocks, latches, WAITs, transações suspensas e eventos pendentes.
- Não reexecutar BIOS após restore.
- Validar ROM, região, revisão de hardware, identidade de BIOS e versões dos estados aninhados.
- A baseline possui snapshots em memória: `SmsWorkerSaveState` versão 1 e estado da máquina versão 33. Codificação em arquivo e slots persistentes são responsabilidades do host.
- Qualquer campo novo persistido exige análise de compatibilidade e versionamento; não mudar versão nesta investigação sem necessidade demonstrada.

## 5. Proveniência inicial por subsistema

| Subsistema | Evidência disponível | Regra do contrato |
|---|---|---|
| Z80 RESET | Manual Zilog: PC/I/R, IFF e IM; mínimo 3 ciclos de clock | Implementar/representar o sinal separado do estado de power-on; não inventar SP |
| Registradores gerais/SP/IX/IY | Não especificados pelo trecho de RESET; valores de BIOS são revisionais | `Unknown` no perfil físico; policy determinística explícita em boot direto |
| Memory Control $3E | Bits de seleção documentados como ativos baixos; bits 1–0 desconhecidos | Valor de partida por modelo/BIOS; não usar $E0 ou $AB como universal |
| BIOS | Executável real, revisão e hash identificáveis | Executar instruções e writes pelo Scheduler |
| Mapper | Sega/Codemasters têm protocolos conhecidos; BIOS não pagina antes da busca | `Known/Unknown/Ambiguous`; nunca inferir por nome/região/tamanho isolado |
| Work RAM | 8 KiB espelhados; conteúdo de power-on não universalmente documentado; BIOS escreve/limpa | Separar conteúdo power-on de efeitos efetivamente escritos pela BIOS |
| VDP | Contadores e timing documentados; estado de registradores/VRAM no power-on não estabelecido pelas fontes consultadas | Defaults atuais são policy até validação por hardware/BIOS |
| PSG | PSG integrado Sega inicia registradores de tom/ruído em zero e volumes em 1/silêncio; fase interna é menos documentada | Preservar valores de registradores; não alterar fase sem evidência |
| I/O | Port $3F controla direção e níveis TH/TR; valor físico de power-on por modelo não confirmado | Modelar input/output e níveis como perfil, não universalizar $FF sem evidência |
| Scheduler | Autoridade temporal do runtime | Boot/BIOS/reset/restore devem manter ordem causal e estado retomável |

## 6. Política de mapper

A política de mapper deve ser independente do boot path.

- Fingerprint por SHA-256 é a identidade preferencial; CRC32 + tamanho só quando a fonte não fornece SHA-256, sempre com proveniência.
- TMR SEGA/SDSC são metadados, não prova isolada de PCB.
- Detecção deve retornar `Known(profile,evidence)`, `Unknown` ou `Ambiguous(candidates)`.
- O fallback atual para Sega deve ser tratado como compatibilidade provisória, não como detecção física.
- Não adicionar exceções por nome de jogo.
- Nenhuma ROM comercial deve ser versionada.

## 7. Matriz de testes associada

1. Cada caminho deve expor um snapshot inicial com provenance.
2. BIOS real: primeiro fetch, primeiro write $3E, efeitos $C000, VDP/PSG/I/O e primeiro fetch do cartucho.
3. Direct cartridge: PC e primeiros fetches, inicialização de SP, primeira CALL/PUSH, mapper antes/depois de FFFC-FFFF.
4. Reset: CPU RESET temporal (3 T-states) separado de reset frio da aplicação e Stop/power-off.
5. LoadRom: troca de ROM sem estado residual e política de SRAM.
6. Save State: snapshots antes/depois do handoff e no meio de M-cycle/WAIT; restore bit-exato e determinístico.
7. Execução contínua versus blocos, Scheduler e saída audiovisual sem feedback de host.
8. Regressões Andre Agassi Tennis, Ace of Aces, Road Rash e Schtroumpfs 2.

## 8. Decisões ainda abertas

- Qual revisão/modelo de SMS é o alvo de cada ROM e qual firmware é compatível.
- Valor exato do latch $3E no início físico por modelo; bits 1–0.
- Conteúdo de RAM/VRAM/registradores VDP em power-on sem BIOS.
- SP e demais registradores na saída de cada BIOS real.
- Se o boot direto deve manter os defaults determinísticos atuais ou exigir uma policy configurável para campos não especificados.
- Mapper real das duas ROMs-alvo após hash e inspeção do dump.
- Se o Reset da aplicação deve permanecer reset frio ou evoluir para um reset de console temporal separado.

Nenhuma dessas lacunas deve ser resolvida com valores copiados de Ares/GearSystem sem comparar o mesmo caminho de boot e o mesmo instante.
