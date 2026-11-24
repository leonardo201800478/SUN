# CHRONOS — avaliação estratégica, SWOT e stakeholders

**Projeto:** SUN V2

**Componente:** CHRONOS — SUN Temporal Core

**Sistema inicial:** Sega Master System (SMS)

**Status:** análise estratégica; recomendações sujeitas a validação técnica

**Documento relacionado:** [CHRONOS_ARCHITECTURE_EVALUATION.md](CHRONOS_ARCHITECTURE_EVALUATION.md)

**Branch:** architecture/sun-v2-foundation

---

## 1. Resumo executivo

A direção do CHRONOS é tecnicamente promissora, mas ainda não está pronta para ser congelada como arquitetura de produção.

A proposta híbrida oferece um caminho plausível para combinar avanço em blocos, sincronização sob demanda e deadlines seletivos. Seu principal risco é transformar essa flexibilidade em três mecanismos concorrentes, com fronteiras de responsabilidade ambíguas, custo de execução elevado e dificuldade de depuração.

A recomendação estratégica é **manter a hipótese híbrida, mas reduzi-la a um híbrido mínimo e comprovável**:

1. uma autoridade única de tempo virtual e de ordenação causal;
2. propriedade do estado interno mantida pelos próprios dispositivos;
3. avanço limitado por deadlines e pontos de observação;
4. sincronização sob demanda somente nos acessos que realmente exigem estado atualizado;
5. agenda explícita apenas para eventos autônomos que precisam acordar ou limitar a execução;
6. suspensão de transações Z80/barramento nos pontos comprovadamente necessários;
7. nenhuma thread por dispositivo ou paralelismo no caminho causal sem prova de independência.

A expressão "autoridade temporal única" não deve ser interpretada como "um objeto central que possui e modifica todo o estado de todos os chips". O Chronos deve possuir o tempo e a política de coordenação; CPU, VDP, PSG, YM2413 e barramentos devem continuar donos de seus estados e invariantes internos. Essa distinção reduz acoplamento e evita transformar o núcleo temporal em um mega-controlador.

**Veredito:** prosseguir com CHRONOS, mas não aprovar ainda o modelo F completo, a estrutura de eventos, a representação numérica ou o contrato final de barramento.

---

## 2. Objetivos estratégicos e critérios de sucesso

O Chronos não deve ser considerado bem-sucedido apenas porque o scheduler é elegante, rápido em microbenchmark ou passa testes sintéticos.

O resultado final precisa demonstrar simultaneamente:

| Dimensão | Resultado desejado | Como comprovar |
|---|---|---|
| Compatibilidade | Jogos SMS executam sem falhas temporais relevantes | ROMs de teste, jogos reais, regressões e comparação diferencial |
| Fidelidade de vídeo | Counters, IRQs, raster, scroll, sprites e acesso a VRAM respeitados | Testes VDP e capturas determinísticas |
| Fidelidade de áudio | PSG/FM contínuos, fase e transições corretas | Testes de chip, comparação PCM e ausência de underruns |
| Determinismo | Mesmas entradas e estado geram a mesma execução | Hashes de estado, replay e restore diferencial |
| Latência | Input chega ao próximo ponto de observação sem filas desnecessárias | Input-to-frame e input-to-photon |
| Desempenho | Throughput suficiente para manter folga e apresentação estável | Benchmarks headless e integração real |
| Manutenibilidade | Regras temporais compreensíveis e testáveis | Revisões, documentação, custo de mudança e bugs por regressão |
| Evolução | Novas máquinas não exigem reescrever o núcleo sem necessidade | Prova posterior com segundo sistema, após SMS |

### 2.1 Separar duas metas de desempenho

Existem duas metas distintas que não devem ser misturadas:

- **60 FPS estáveis na apresentação:** requisito de experiência em tempo real, com frame pacing e margem para o host.
- **120 FPS de throughput headless com YM2413 ativo em OutRun:** meta de desempenho registrada na iniciativa PERF-01, a ser medida sem pacing artificial.

A meta de 120 FPS não significa que todo jogo, toda configuração ou toda plataforma precise apresentar 120 FPS. Tampouco prova baixa latência. O throughput pode ser alto e o input-to-photon continuar ruim se houver filas ou frames em voo.

A arquitetura deverá atender à fidelidade primeiro e demonstrar desempenho por medições integradas, não por simplificação temporal.

---

## 3. Matriz SWOT

### 3.1 Strengths — Forças

| Força | Valor estratégico | Condição para preservar |
|---|---|---|
| Autoridade temporal explícita | Evita clocks independentes e deriva entre componentes | Contratos claros de avanço e timestamps |
| Separação entre tempo, agendamento e granularidade | Permite trocar mecanismo sem reescrever hardware | Não fundir as três dimensões em uma abstração monolítica |
| Suspensão/retomada de barramento | Abre caminho para WAIT states e microtiming real | Estado parcial serializável e efeitos exatamente uma vez |
| Determinismo como requisito | Facilita replay, regressões e diagnóstico | Eliminar dependência de ordem incidental e relógio do host |
| Avaliação por protótipos | Evita escolher arquitetura por preferência | Protótipos com mesma semântica e cargas comparáveis |
| Separação do host | Protege o clock virtual de VSync, CPAL e atrasos do SO | Filas de apresentação e áudio fora do caminho temporal causal |
| SMS como primeiro alvo | Limita o problema e permite validar com testes conhecidos | Não generalizar antes de conhecer o hardware SMS |

### 3.2 Weaknesses — Fraquezas

| Fraqueza | Consequência | Tratamento recomendado |
|---|---|---|
| Modelo híbrido pode acumular mecanismos | Mais estados, caminhos e bugs de integração | Adotar híbrido mínimo; cada evento deve ter um único dono |
| R1 depende de fatos de hardware ainda incertos | Especificação pode formalizar premissas erradas | Registrar incerteza, revisão de hardware e evidência por sinal |
| Transações suspensíveis aumentam complexidade | CPU e barramento ficam mais difíceis de testar | Começar por uma classe de ciclo e expandir por necessidade demonstrada |
| Protótipos podem consumir tempo sem reduzir incerteza | Atraso do projeto | Fazer benchmark por decisão; descartar protótipo sem hipótese mensurável |
| Métricas de latência atravessam vários subsistemas | Chronos pode receber culpa por gargalo do host | Instrumentar pipeline completo e atribuir cada intervalo ao responsável |
| Save state exige persistir estado pendente | Evolução do formato fica mais difícil | Definir versão, migração e testes de compatibilidade desde o primeiro formato |
| Sete frentes de análise não equivalem a sete revisores independentes | Risco de falsa confiança | Separar análise interna e parecer externo adversarial |

### 3.3 Opportunities — Oportunidades

| Oportunidade | Benefício futuro | Pré-condição |
|---|---|---|
| Modelo temporal reutilizável | Reduzir custo para SG-1000 e futuras máquinas Sega | Isolar políticas específicas de cada máquina |
| Replay determinístico | Regressões reproduzíveis e diagnóstico de bugs raros | Entrada registrada com timestamp e estado completo |
| Ferramentas de conformidade temporal | Detectar divergências de CPU/VDP/áudio antes de gameplay | Modelo de referência independente |
| Medição de input-to-photon | Diferenciar baixa latência real de simples FPS alto | Instrumentação no host e apresentação |
| Escalonamento seletivo de precisão | Pagar custo fino somente nos pontos necessários | Perfil de eventos e limites de avanço |
| Save states robustos | Continuação consistente em qualquer ponto permitido | Snapshot de transações e fases pendentes |
| Extensão para outras arquiteturas | Possível reaproveitamento de primitivas temporais | Segundo sistema piloto e prova de generalidade |

### 3.4 Threats — Ameaças

| Ameaça | Impacto | Mitigação |
|---|---|---|
| Fontes de hardware incompletas ou contraditórias | Fase e ordenação erradas | Triangular manual, testes físicos conhecidos e emuladores de referência |
| Microtiming não modelado | Jogos específicos falham apesar de boa média | Testes dirigidos a acessos, IRQs, WAIT e raster |
| Complexidade do híbrido supera seu ganho | Baixo throughput e alto custo de manutenção | Comparar com modelos simples e impor teto de mecanismos |
| Otimização prematura | Regressões temporais difíceis de detectar | Baseline antes/depois, mudança isolada e teste diferencial |
| Acoplamento ao SMS | Chronos não serve a outros sistemas | Núcleo genérico de tempo; perfis e semânticas nos adaptadores de máquina |
| Mudança de escopo para SG-1000/Genesis cedo demais | SMS nunca fecha | Gate de conclusão SMS antes de expansão |
| Dependência de um único mantenedor | Conhecimento tácito e risco de abandono | ADRs, diagramas, contratos, testes executáveis e guias de contribuição |
| Confusão entre FPS, VPS e latência | Decisões de arquitetura baseadas em métricas incompatíveis | Definições, metodologia e condições de teste fixadas |
| Pressão por feature antes de qualidade | Precisão e estabilidade sacrificadas | Bloqueio formal de novas features quando houver regressão de performance ou fidelidade |

### 3.5 Leitura estratégica da SWOT

A força mais importante não é a hipótese híbrida; é a separação explícita entre representação temporal, agendamento, granularidade e causalidade. Essa separação permite mudar de ideia sem reescrever todos os dispositivos.

A fraqueza mais perigosa é a complexidade de coordenação. Ameaça mais séria é formalizar fases de hardware incorretas. A maior oportunidade é construir um núcleo temporal reutilizável, mas somente depois de provar o caso SMS.

---

## 4. Stakeholders humanos e organizacionais

Os stakeholders são avaliados por poder de decisão, interesse, impacto e estratégia de envolvimento. O projeto atualmente tem um mantenedor principal; portanto, a matriz também evidencia risco de concentração de conhecimento.

| Stakeholder | Interesse principal | Poder/influência | Risco ou contribuição | Estratégia |
|---|---|---:|---|---|
| Mantenedor/arquitetura SUN | Fidelidade, desempenho, escopo e direção técnica | Muito alto | Decisões rápidas, mas risco de viés e concentração | ADRs, critérios de veto e decisões baseadas em evidência |
| Jogadores/usuários finais | Compatibilidade, resposta, áudio, vídeo e facilidade de uso | Médio, influência indireta alta | Revelam problemas que testes sintéticos não encontram | Matriz de jogos reais, feedback reproduzível e telemetria opcional |
| Revisores independentes | Encontrar falhas e premissas inválidas | Alto no gate de revisão | Reduzem viés de confirmação | Envolver antes de R5, com acesso a especificação e evidências |
| Contribuidores futuros | Código compreensível, testes e contratos estáveis | Médio | Podem acelerar evolução ou introduzir divergência | Documentação, guidelines, CI e ownership claro |
| Mantenedores de referências upstream | Correções e conhecimento sobre emulação | Baixo poder sobre SUN, alta influência técnica | Evidência valiosa, mas código não é prova física | Citar fontes, comparar versões e não copiar sem validação |
| Autores de testes/ROMs de diagnóstico | Cobertura de hardware e casos extremos | Médio | Podem expor limites não cobertos por jogos comerciais | Manter suíte de conformidade e registrar versão/hash |
| Usuários de plataformas/hosts | Compatibilidade com Windows e futuros SOs | Médio | Driver, VSync e áudio podem mascarar problemas do core | Testes por backend e separação host/core |
| Distribuidores/empacotadores futuros | Releases reproduzíveis e suporte | Baixo no início | Exigem estabilidade e formatos versionados | Só envolver em fase de produto/release |

### 4.1 Matriz poder × interesse

| Grupo | Poder | Interesse | Tratamento |
|---|---|---|---|
| Mantenedor principal | Alto | Alto | Gerenciar de perto |
| Revisor independente | Alto | Alto | Gerenciar de perto nos gates R1/R4/R5 |
| Jogadores/testadores | Médio | Alto | Consultar continuamente após protótipos jogáveis |
| Contribuidores | Médio | Alto | Manter informados e envolvidos em contratos |
| Comunidade de emulação/upstream | Baixo a médio | Médio/alto | Consultar com perguntas técnicas específicas |
| Plataformas/hosts | Médio | Médio | Validar por matriz de compatibilidade |
| Distribuição/release | Baixo inicialmente | Crescente no fim | Envolver na preparação de produto |

### 4.2 Stakeholders que não podem ser esquecidos

- **Jogador:** a medida final não é a beleza do scheduler; é o jogo responder corretamente.
- **Revisor independente:** revisão interna não substitui contestação externa.
- **Futuro mantenedor:** se somente o autor original entende as fases e os contratos, a arquitetura não está suficientemente documentada.
- **Host/driver:** o core pode estar correto e a experiência ainda ter latência por filas de vídeo ou áudio.
- **Comunidade de testes:** casos de hardware devem ser rastreáveis por ROM, versão e resultado.

---

## 5. Stakeholders técnicos: componentes e dependências

Esta matriz trata os subsistemas como stakeholders técnicos: cada um depende do contrato temporal e pode ser prejudicado por decisões feitas no Chronos.

| Componente | O que exige do Chronos | O que pode comprometer | Contrato recomendado |
|---|---|---|---|
| Z80 | T-states, M-cycles, WAIT, IRQ/NMI e suspensão | Instrução atravessar deadline ou efeito duplicado | CPU solicita avanço; barramento reporta fases e conclusão |
| Barramento | Timestamp de início, amostragem e efeito | Leitura/escrita aplicada fora da fase | Transação explícita, retomável e exatamente uma vez |
| RAM/VRAM | Acesso causal e contenção quando aplicável | Catch-up tardio ou escrita antecipada | Acesso sincroniza dependência antes de observar/aplicar |
| VDP | Dot/linha, counters, IRQ e acesso a memória | Avanço além de mudança observável | Advance-to-time limitado por deadline e estado próprio |
| PSG | Clock, divisor e fase contínua | Amostras/transições deslocadas | Avanço por intervalo com fase preservada |
| YM2413 | Clock, slots, divisores e continuidade | FM dessincronizado ou custo excessivo | Domínio próprio sincronizado com tempo virtual |
| Mixer/resampler | Timestamp e continuidade de blocos | Áudio quebrado ou acoplado ao callback | Processamento de blocos fora da autoridade temporal |
| Input | Fronteira de aplicação da entrada | Input atrasado por fila/frame | Timestamp de entrada e política de aplicação explícita |
| Vídeo/WGPU | Frame pronto e timestamp de publicação | Backpressure ou frame antigo | Consumo de snapshot/frame sem controlar tempo virtual |
| Áudio/CPAL | Blocos PCM disponíveis | Underrun e bloqueio do core | Ring buffer e política de starvation no host |
| Save state | Estado temporal completo | Restore divergente | Snapshot versionado, sem ponteiros e com estado pendente |
| Debug/telemetria | Eventos e timestamps observáveis | Instrumentação altera hot path | Tracing opcional e buffers de observabilidade desacoplados |

### 5.1 Decisão importante: Chronos não deve possuir o estado de todos os chips

A expressão "autoridade temporal única" precisa ser definida com rigor:

- Chronos possui a noção de tempo virtual confirmado, deadlines e ordenação causal.
- Cada dispositivo possui seus registradores, contadores internos, fase local e regras de evolução.
- O barramento possui o estado das transações e efeitos pendentes.
- A CPU possui o estado arquitetural e a microsequência necessária para suspender/retomar.
- O host possui filas e relógios físicos, sem autoridade sobre o tempo virtual.

O Chronos coordena e valida limites; não deve virar um objeto que conhece os detalhes internos de cada chip. Isso preserva modularidade e permite testar cada dispositivo isoladamente.

---

## 6. Avaliação fria: o que deve mudar na proposta atual

### Mudança 1 — Trocar "híbrido" por "híbrido mínimo" como hipótese

**Problema:** o modelo F atual combina avanço em blocos, sincronização sob demanda, deadlines, agenda seletiva e CPU suspensível. Sem limites, pode virar um scheduler central com vários mecanismos redundantes.

**Alteração:** o modelo híbrido mínimo terá somente três mecanismos temporais explícitos:
1. avanço limitado até um timestamp seguro;
2. sincronização sob demanda em pontos de observação;
3. agenda de deadlines autônomos quando o dispositivo precisa interromper ou acordar a execução.

Suspensão de barramento é contrato da CPU/barramento, não um quarto scheduler. Corrotinas são uma possível técnica de implementação, não uma política temporal obrigatória.

**Critério:** cada mecanismo deve demonstrar uma classe de problema que os outros dois não resolvem com igual custo.

### Mudança 2 — Separar autoridade temporal de propriedade de estado

**Problema:** "proprietário único do estado causal" pode ser interpretado como centralizar mutação de todos os chips, criando acoplamento e gargalo.

**Alteração:** Chronos é o único árbitro do tempo virtual e da ordem de coordenação; os dispositivos mantêm propriedade do próprio estado. Efeitos cruzados passam por contratos explícitos.

**Critério:** nenhum módulo pode alterar o timestamp de outro ou produzir efeito retroativo, mas cada dispositivo deve poder ser testado sem depender de uma instância global de Chronos.

### Mudança 3 — Não exigir seis protótipos completos por padrão

**Problema:** implementar P-A a P-F integralmente pode ser custo excessivo, especialmente se uma decisão puder ser eliminada por análise ou microprotótipo.

**Alteração:** executar protótipos em funil:
- primeiro P-A como baseline e P-D/P-E como alternativas centrais;
- P-F somente se a combinação for necessária;
- P-B apenas se a granularidade por quantum permanecer candidata real;
- P-C somente se corrotinas trouxerem benefício conceitual que justifique sua serialização e custo.

O conjunto final deve ser decidido por perguntas de arquitetura, não por obrigação de completar uma lista.

**Critério:** cada protótipo precisa ter hipótese, cenário discriminante, métrica e regra de descarte definidos antes de codificar.

### Mudança 4 — Introduzir uma matriz de evidência por requisito

**Problema:** uma fonte documental pode confirmar frequência nominal sem confirmar fase, jitter, revisão de hardware ou momento de amostragem.

**Alteração:** cada requisito temporal deve conter:
- afirmação;
- fonte primária;
- revisão de hardware;
- nível de confiança;
- divergência conhecida;
- impacto se estiver errado;
- teste que pode confirmar/refutar.

**Critério:** nenhum valor incerto entra como constante imutável da API.

### Mudança 5 — Criar orçamento de complexidade e performance

**Problema:** o documento tem métricas, mas ainda não define limite de custo aceitável por mecanismo.

**Alteração:** antes de R3, estabelecer:
- baseline headless com ROM e hash fixos;
- custo do Chronos separado dos dispositivos;
- dispatches/sincronizações por frame;
- alocações por frame;
- p50/p95/p99;
- pior caso e jitter;
- custo adicional máximo aceitável frente ao baseline;
- margem para crescimento de hardware.

Não fixar números arbitrários antes da baseline. Os limites devem derivar de medições e da folga necessária para 60 FPS estáveis e para a meta headless PERF-01.

### Mudança 6 — Planejar o formato de save state desde R1

**Problema:** snapshot aparece como requisito, mas migração de versão e compatibilidade futura ainda não têm política suficiente.

**Alteração:** separar estado interno de runtime do formato persistido; incluir versão, perfil de máquina, identificação do modelo e regras de migração/rejeição.

**Critério:** um snapshot antigo deve ser restaurado por migração validada ou rejeitado claramente; nunca interpretado silenciosamente com semântica nova.

### Mudança 7 — Definir fronteira de generalização

**Problema:** começar pelo SMS e projetar desde já para todos os consoles pode gerar abstração excessiva; projetar somente para SMS pode bloquear SG-1000 e futuras máquinas.

**Alteração:** Chronos deve conter primitivas genéricas de tempo, deadlines, ordenação e snapshot. A semântica de CPU, VDP, áudio e barramento pertence a adaptadores/perfis da máquina.

Não adicionar requisitos de Mega Drive/Genesis ao R1 do SMS. Depois que SMS estiver validado, usar SG-1000 como segundo caso de generalização se ele for tecnicamente adequado; só então avaliar máquinas mais diferentes.

**Critério:** nenhuma abstração genérica sem dois casos concretos ou uma necessidade comprovada do SMS.

### Mudança 8 — Tornar input-to-photon um requisito de sistema, não promessa do Chronos

**Problema:** o Chronos não controla polling, filas de frame, VSync ou apresentação. Prometer baixa latência apenas pelo scheduler é incorreto.

**Alteração:** o Chronos fornece timestamps e pontos de aplicação de input; o pipeline de host mede captura → aplicação → publicação → apresentação.

**Critério:** latência medida ponta a ponta, com profundidade de fila e número de frames em voo registrados.

---

## 7. Roadmap estratégico: começo, meio e fim

### Fase I — Começo: provar o problema e reduzir incerteza

**Objetivo:** decidir o que precisa ser construído antes de escrever runtime.

1. Fechar inventário de fontes e discrepâncias de clock.
2. Mapear pontos de observação CPU↔bus↔VDP e IRQ.
3. Definir fases conhecidas e desconhecidas do SMS.
4. Especificar contratos mínimos de avanço, sincronização e transação.
5. Definir modelo de referência independente.
6. Estabelecer baseline de desempenho e latência do ambiente.
7. Fazer revisão adversarial da especificação.

**Saída:** R0–R2 aprovados, com incertezas explicitamente classificadas.

**Não fazer:** implementar scheduler de produção, otimizar estrutura de dados ou abrir escopo para outras máquinas.

### Fase II — Meio: provar custo e integração

**Objetivo:** comparar modelos equivalentes e integrar a solução vencedora ao SMS.

1. Prototipar baseline e alternativas discriminantes.
2. Executar testes de causalidade, simultaneidade, WAIT e snapshot.
3. Comparar custo em carga sintética e jogos reais.
4. Integrar primeiro o Z80/barramento com transações retomáveis.
5. Integrar VDP e suas fronteiras de observação.
6. Integrar PSG e YM2413 preservando fase.
7. Testar NTSC e PAL.
8. Medir frame pacing, throughput, áudio e input-to-photon.
9. Executar revisão independente e registrar ADR.

**Saída:** R3–R5 aprovados e Chronos integrado à primeira máquina SMS.

**Não fazer:** paralelizar dispositivos sem evidência, iniciar JIT ou adicionar outro console antes de fechar regressões.

### Fase III — Fim: estabilizar e tornar sustentável

**Objetivo:** provar que o núcleo funciona por longos períodos, é recuperável e pode ser mantido.

1. Testar sessões longas e overflow.
2. Validar save state/restore em estados pendentes.
3. Executar replay determinístico e regressões automatizadas.
4. Testar jogos representativos, não somente ROMs sintéticas.
5. Validar Windows x64 e, depois, outros hosts suportados.
6. Medir estabilidade de áudio e vídeo em carga real.
7. Documentar contratos, limites e diagnóstico de falhas.
8. Criar política de versionamento de snapshots e compatibilidade.
9. Fazer piloto de generalização para o segundo sistema somente após SMS estável.
10. Estabelecer política de regressão: qualquer alteração temporal executa a suíte pertinente.

**Saída:** componente estável, documentado, observável e evolutivo; pronto para servir de base a novas máquinas.

**Não fazer:** declarar conclusão somente por compilação ou benchmark isolado.

---

## 8. Gates de decisão e critérios de parada

| Gate | Pergunta que precisa ser respondida | Se falhar |
|---|---|---|
| G0 — Evidência | Sabemos o que o hardware exige e o que ainda é incerto? | Continuar pesquisa; não especificar como fato |
| G1 — Contratos | CPU, barramento e dispositivos conseguem expressar as fases necessárias? | Redesenhar contratos antes do protótipo |
| G2 — Referência | Conseguimos detectar causalidade e divergência sem usar o algoritmo final como oráculo? | Corrigir modelo de referência |
| G3 — Custo | O modelo atende à precisão sem consumir a folga de performance? | Simplificar mecanismo ou revisar hipótese |
| G4 — Integração | Jogos reais, áudio e vídeo continuam corretos? | Bloquear expansão e corrigir regressão |
| G5 — Sustentabilidade | Restore, replay, sessões longas e manutenção são viáveis? | Não declarar estável |

### Critérios de parada imediata

- Qualquer perda de causalidade ou efeito duplicado.
- Divergência determinística após restore.
- Regressão de compatibilidade em ROM de referência sem explicação.
- Underruns recorrentes atribuíveis à arquitetura temporal.
- Queda abaixo da meta de frame pacing em carga antes sustentável.
- Crescimento de complexidade sem ganho mensurável.
- Dependência de timing do host para fazer o hardware virtual funcionar.

Quando o problema for performance, interromper novas features e medir o gargalo antes de alterar semântica.

---

## 9. Métricas de governança do projeto

Além das métricas de runtime, acompanhar:

| Indicador | O que revela |
|---|---|
| Requisitos com fonte e confiança documentadas | Qualidade da base técnica |
| Invariantes com teste executável | Cobertura de propriedades essenciais |
| Bugs temporais por subsistema | Concentração de risco |
| Regressões introduzidas por mudança | Estabilidade da arquitetura |
| Tempo médio para reproduzir bug | Qualidade de observabilidade |
| Tempo de revisão independente | Clareza e auditabilidade |
| Custo de integração de um dispositivo | Acoplamento do Chronos |
| Alterações necessárias para segundo sistema | Generalidade real |
| Custo por frame e por sincronização | Sustentabilidade de performance |
| Restore/replay divergentes | Integridade temporal |

Evitar usar quantidade de commits, linhas de código ou número de testes como indicadores isolados de progresso.

---

## 10. Decisões que devem permanecer abertas

Até existir evidência, manter em aberto:
- u64, u128, racional ou representação híbrida;
- heap, vetor, árvore, timing wheel ou agenda local;
- corrotinas versus execução explícita;
- quantum fixo versus deadline variável;
- granularidade mínima do Z80 e do VDP;
- fases de eventos simultâneos;
- paralelismo por dispositivo;
- formato definitivo do save state;
- limites numéricos de overhead.

Uma decisão aberta não é atraso: é controle de risco enquanto falta evidência.

---

## 11. Veredito final

**Prosseguir com CHRONOS. Alterar a forma de conduzir a arquitetura, não abandonar a direção.**

A proposta deve evoluir de "vamos construir um scheduler híbrido" para:

> "Vamos construir o menor mecanismo temporal determinístico que consiga provar causalidade, fidelidade de hardware, baixo custo e capacidade de evolução no SMS."

As três prioridades mais importantes são:

1. **Precisão demonstrável:** clocks, fases, barramento e simultaneidade.
2. **Simplicidade operacional:** autoridade temporal única, estado pertencente aos dispositivos e mecanismos mínimos.
3. **Desempenho e experiência real:** benchmark integrado, 60 FPS estáveis, throughput headless medido e input-to-photon instrumentado.

O Chronos só será uma boa arquitetura se, no fim, for possível explicar seu comportamento, provar suas propriedades, restaurar seu estado e adicionar um novo dispositivo sem reescrever o núcleo.

**Decisão recomendada para a próxima etapa:** atualizar a avaliação arquitetural com o conceito de híbrido mínimo e fechar a matriz de evidências de R0. Não iniciar implementação de produção antes de R5.
