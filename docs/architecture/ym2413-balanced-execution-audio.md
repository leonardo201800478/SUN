# YM2413 — estratégia equilibrada de execução, saída e desempenho

**Data:** 2026-11-18
**Estado:** proposta arquitetural complementar; não substitui o modelo canônico de hardware nem declara síntese FM concluída.
**Referências canônicas:** [modelo YM2413](ym2413-fm-model.md) e [scorecard de conclusão FM](../ym2413-fm-completion-scorecard.md).
**Objetivo:** preservar precisão temporal documentada sem executar trabalho redundante por clock mestre nem introduzir filtros/ganhos de placa sem evidência.

## 1. Decisão executiva

Adotar um modelo **orientado a eventos temporais**, com execução serial por slot, estado integral por operador e saída de áudio reamostrada por FIR polifásico.

Evitar os dois extremos:
- Não executar uma rotina completa de síntese em cada um dos 3.579.545 clocks por segundo.
- Não reduzir o YM2413 a um sintetizador de 49,7 kHz sem preservar as transições temporais dos pinos MO/RO.

O custo-alvo do núcleo digital é uma visita de operador por slot: 18 visitas por rotação de 72 clocks. Isso equivale a aproximadamente 894.886 eventos de slot/s no clock nominal. É uma carga derivada da organização documentada, não um benchmark de CPU.

Regra central: **pular clocks sem eventos internos comprovados; nunca pular evento observável ou fronteira temporal necessária ao estado**.

## 2. Domínios temporais

No clock nominal de 3.579.545 Hz:

| Domínio | Cadência | Uso |
|---|---:|---|
| Clock de entrada YM2413 | 3.579.545 Hz | Referência temporal e timestamps de escrita |
| Slot | 4 clocks (1,117 µs) | Seleção serial de um operador |
| Rotação completa | 72 clocks (20,114 µs) | 18 slots; saída aproximada de 49.715,90 Hz |
| Eventos de slot | 894.886,25/s | Orçamento de eventos do pipeline |
| PSG NTSC | 3.579.545 / 16 = 223.721,56 Hz | Tick digital PSG conforme documentação do projeto |
| Fonte SMS atual | 48.000 Hz | Contrato observado no worker SMS auditado |
| Host | taxa negociada | Saída CPAL |

A taxa de rotação correta é 3.579.545 / 72 = 49.715,90278 Hz. Não confundir taxa de slot com taxa de saída.

O PSG deve permanecer no contrato atual de 48 kHz nesta fase. A especificação de áudio anexada propõe oversampling do PSG em 223,72 kHz seguido de decimação, mas a auditoria do runtime encontrou o worker SMS entregando PCM em 48 kHz. Alterar esse caminho agora ampliaria o escopo e poderia regredir áudio já funcional. Oversampling PSG fica como melhoria separada, sujeita a medição.

## 3. Núcleo YM2413: execução por eventos

### 3.1 Não usar loop por clock mestre

Evitar uma iteração de software para cada input clock. Isso faria milhões de iterações por segundo mesmo quando não existe operação interna comprovada naquele clock.

### 3.2 Avanço até a próxima fronteira

Manter contador monotônico de clocks de entrada e fase residual. O adaptador solicita avanço até um deadline do Scheduler. O core calcula quantos clocks pode consumir até o próximo evento conhecido:
- fechamento do slot atual;
- fechamento da rotação;
- escrita de registrador com timestamp;
- transição de saída MO/RO conhecida;
- outro evento interno somente quando sua cadência estiver demonstrada.

O estado deve preservar elapsed_input_clocks, slot_index e input_clock_phase. O resultado deve ser idêntico para advance(N) e para qualquer partição cuja soma seja N.

Pseudocódigo conceitual:

```rust
while remaining != 0 {
    let clocks = clocks_until_next_known_boundary(remaining, state);
    advance_monotonic_clock(clocks);
    remaining -= clocks;

    if slot_boundary_reached(state) {
        process_one_slot_using_verified_order(state);
    }
    if rotation_boundary_reached(state) {
        process_global_rotation_events(state);
    }
}
```

Não inserir ações arbitrárias nos quatro clocks do slot. A investigação P1 concluiu que a ordem serial está documentada, mas offsets internos PG/EG/operador/DAC dentro das quatro posições ainda não foram estabelecidos. Até fechar essa evidência, esses pontos são fronteiras de instrumentação, não subfases inventadas.

### 3.3 Uma célula de operador, 18 estados lógicos

Manter 18 acumuladores PG, 18 estados EG e nove históricos de feedback. Executar uma única rotina de operador reutilizada sequencialmente conforme YM2413_SLOT_MAP.

Isso reduz duplicação de execução, reproduz a organização serial, preserva estado por operador para snapshots e facilita instrumentação. Os arrays de estado são necessários; 18 pipelines de execução simultâneos não são.

### 3.4 Aritmética e tabelas

No hot path do core:
- inteiros com largura explícita para PG, EG, feedback e fase;
- tabelas Log-Sine/EXP pré-calculadas;
- sem sin, pow, log ou ponto flutuante por operador;
- saturação, truncamento e deslocamentos explícitos;
- sem alocação dinâmica, mutex ou I/O no tick de chip.

A conversão para f32 fica na fronteira de saída/DSP. Não substituir aritmética inteira por ponto flutuante apenas por conveniência.

## 4. Saída MO/RO: precisão sem oversampling global

### 4.1 Preservar eventos de borda

A pesquisa publicada descreve MO/RO multiplexados ao longo de 72 clocks, com janela ativa de três clocks e retorno ao nível neutro. Isso exige preservar largura e posição temporal dos pulsos, mas não executar toda a síntese FM em cada clock mestre.

Representar a saída como eventos timestamped: instante de início, linha MO ou RO, valor digital/DAC associado e instante de retorno ao nível neutro. Os valores DAC e o roteamento exato precisam seguir evidência validada. A tabela de slots de Rhythm Mode, isoladamente, não define todo o roteamento de saída.

### 4.2 Modelo elétrico por avanço analítico

Quando o perfil de placa estiver documentado, filtros RC podem avançar entre eventos com solução analítica da resposta exponencial. Assim, um pulso de três clocks é preservado sem amostrar o circuito 3,58 milhões de vezes por segundo.

Para um RC passa-baixa de primeira ordem com entrada constante x durante Δt:

\[y(t+\Delta t)=x+(y(t)-x)e^{-\Delta t/\tau}\]

O cálculo pode usar coeficiente pré-calculado por intervalo discreto de clocks ou tabela de coeficientes. A escolha entre f64, f32 e fixed-point será decidida por teste de erro e benchmark. O filtro não entra no core digital.

A equação descreve um RC ideal; não certifica que uma placa Japanese SMS específica use esse RC, valores ou topologia. Os filtros/ganhos sugeridos anteriormente permanecem perfis experimentais, não defaults.

### 4.3 Separação dos ramos

Manter MO e RO independentes no modelo do chip. Não somá-los com ganho arbitrário dentro de native/ym2413. Soma elétrica, carga, amplificação e acoplamento pertencem ao perfil de placa.

## 5. Reamostragem e mixagem com PSG

### 5.1 Taxas e fronteira de mixagem

A fonte FM deriva de 49.715,90278 Hz; a fonte PSG atual do worker SMS é 48 kHz. O mixer existente requer fontes com o mesmo número de frames. A solução de menor risco é:

1. gerar FM na cadência nativa;
2. aplicar o modelo de saída da placa FM, se habilitado;
3. reamostrar FM para a linha de tempo PCM de 48 kHz do worker SMS;
4. alinhar FM ao mesmo intervalo de emulação do bloco PSG;
5. misturar PSG + FM uma única vez;
6. passar o resultado pela política de saída existente e pelo resampler final para a taxa do host.

Não chamar NativeAudioSession::process_samples separadamente para PSG e FM: a implementação atual constrói uma única fonte por chamada; chamadas consecutivas publicariam blocos em sequência, não em paralelo temporal.

A extensão futura deve aceitar várias fontes mono alinhadas, com ganhos explícitos, numa única operação de mixagem. Preservar a API atual e adicionar caminho multi-source testado.

### 5.2 FIR polifásico

Para converter aproximadamente 49,716 kHz em 48 kHz, avaliar como ponto inicial um FIR polifásico com 32 taps, 256 fases, coeficientes normalizados, histórico fixo e fase racional/fixed-point persistente. Esses números são uma configuração de engenharia, não uma propriedade do YM2413.

Confirmar a configuração por resposta em frequência, rejeição de aliasing, erro de amplitude, custo de CPU e equivalência entre blocos. Não usar interpolação linear como solução final de anti-aliasing; interpolação cúbica também não garante banda limitada. Um modo simples pode existir para depuração, rotulado como não band-limited.

### 5.3 Buffers

No core/adaptador por slot, usar arrays fixos e buffers pré-alocados. Na camada de áudio, reutilizar scratch buffers e adicionar APIs process_into para resampler/mixer quando compatível. A sessão atual usa Vec e o resampler devolve novo Vec; não declarar zero-allocation end-to-end até isso ser medido e melhorado.

## 6. Precisão temporal e determinismo

O desenho precisa satisfazer:

1. advance_input_clocks(N) independente da partição em blocos.
2. Escritas aplicadas no timestamp correto, sem deslocamento para começo/fim do frame.
3. Esperas de barramento de 12/84 clocks do manual respeitadas no adaptador.
4. Eventos MO/RO preservando timestamps e duração.
5. Fase FIR, histórico, estado analógico e último evento de saída incluídos no snapshot agregado.
6. Restore + continuação produzindo a mesma sequência de eventos e PCM.
7. MachineScheduler como única autoridade temporal.
8. CPAL consumindo PCM pronto, sem avançar o chip.

## 7. Validação em camadas

### Gate A — temporal do core
- avanço em blocos aleatórios versus contínuo;
- wrap de slot e rotação;
- ordem completa dos 18 slots;
- timestamps de escrita ao redor das fronteiras;
- overflow e restore.

### Gate B — síntese serial
- operador isolado por slot;
- modulator/carrier com feedback e histórico explícito;
- PG/EG atualizados apenas nos pontos sustentados por evidência;
- comparação por vetor contra referência versionada;
- divergências registradas, não corrigidas por aproximação sonora.

### Gate C — saída multiplexada
- pulsos de três clocks e intervalo neutro;
- eventos simultâneos MO/RO;
- largura e ordem após save/restore;
- comparação com traços publicados ou captura física.

### Gate D — DSP
- impulso, degrau, senoides e tons próximos de Nyquist;
- resposta FIR e perfis analógicos;
- rejeição de aliasing;
- continuidade nas fronteiras de bloco;
- equivalência contínua/em blocos.

### Gate E — integração SMS
- alinhamento de blocos PSG/FM no mesmo intervalo de máquina;
- mixagem simultânea, sem concatenação;
- ausência de regressão em SMS sem FM e SG-1000;
- save state, pause/resume, reset e StepFrame;
- ROMs de música/efeitos e cenários PCM via PSG.

## 8. Benchmark e critérios de desempenho

Criar benchmarks reprodutíveis para custo de rotação (18 slots), segundo de emulação, FIR, filtro analógico por evento, mixagem PSG+FM, alocações por bloco e latência. Registrar CPU, compilador, target, flags, duração, mediana e p95; comparar builds release no mesmo runner.

Não adotar SIMD, paralelismo, cache de lookup ou redução de precisão antes de demonstrar gargalo. A rotação de 18 slots é pequena e sequencial; paralelizá-la adicionaria overhead e poderia destruir dependências de feedback.

Primeiro criar baseline. Depois, regressões superiores a 10% no mesmo ambiente exigem explicação ou correção. Nenhum ganho de desempenho justifica divergência temporal ou perda de estado.

## 9. Sequência de implementação

| Etapa | Entrega | Não fazer ainda |
|---|---|---|
| B1 | **Concluída:** PR #1024 integrada em `f3480e453e32b6c4ce2d80314c8163c405d1f2ff`; correções EG zero-rate hold/DAMP e atualização do scorecard | Tratar isso como síntese PCM concluída |
| B2 | **Contrato integrado:** PR #1124 codificou o contrato serial de slots em `8a80fc784c3b9b2563cea248f5c3ba1b4073dfc9`; hooks internos de core seguem em revisão na PR #1133 | Iniciar serviço PG serial ou presumir offsets internos fechados antes de resolver a revisão |
| B3 | Integrar síntese serial por slot somente após fechamento dos hooks, offsets sustentados e validação temporal | Executar 18 operadores em paralelo |
| B4 | Implementar MO/RO como eventos com duração | Filtro de placa não validado |
| B5 | FIR polifásico e testes DSP independentes | Interpolação linear como anti-aliasing final |
| B6 | Mixagem multi-source alinhada em 48 kHz | Chamar sessão duas vezes por frame |
| B7 | Perfil Japanese SMS e snapshots | Alterar PSG/SG-1000 ou Scheduler sem necessidade |

## 10. Pendências que permanecem explícitas

- offsets internos PG/EG/OP/feedback nos quatro clocks do slot;
- fase inicial dos acumuladores e contador global;
- reset/enable e cadência física do LFSR de ritmo;
- latch/roteamento exato do DAC e seleção MO/RO por slot;
- topologia elétrica da revisão Japanese SMS;
- relação de ganho FM/PSG;
- taxa final PCM e política de perfil analógico por placa.

**Resultado esperado:** execução barata por eventos, estado digital determinístico, pulsos de saída preservados, DSP de banda limitada e integração alinhada. Isso oferece equilíbrio mensurável entre desempenho e fidelidade sem fingir que as incertezas do silício já foram resolvidas.


## Revalidação de continuidade — 2026-11-21

**Main consultada:** `344d56e03e668a269f24e933dd33876e0bb9bf27`.

- PR #1024 está integrada; a etapa B1 desta proposta não é mais pendência.
- PR #1124 está integrada e codifica o contrato de serviço serial por slot. Isso não implementa o serviço PG nem a síntese FM.
- PR #1133 permanece aberta para revisão dos hooks internos de slot. A aprovação/revisão técnica e os gates associados continuam bloqueando a implementação seguinte; esta proposta não os substitui.
- B3–B7 permanecem pendentes. Não declarar PCM FM contínuo, operadores completos, Rhythm Mode, perfil SMS ou mixagem PSG+FM.
- O modelo de hardware em `ym2413-fm-model.md` e o scorecard são as fontes canônicas para evidência e estado. Este documento é uma proposta de execução/DSP; parâmetros de FIR e filtros analógicos permanecem propostas de engenharia, não propriedades confirmadas do chip ou de uma placa específica.
