# SUN V2 — Fundação arquitetural

**Status:** proposta inicial; sujeita à pesquisa do Temporal Core e aprovação antes da implementação.

## 1. Objetivo

Construir uma plataforma de emulação orientada à execução real de jogos. A arquitetura deve controlar custos de processamento sem remover estados ou efeitos de hardware que possam ser observados pelo software emulado.

## 2. Princípios não negociáveis

1. **Tempo virtual único:** a máquina tem uma linha temporal coerente. Dispositivos não avançam clocks por conta própria fora do mecanismo temporal acordado.
2. **Causalidade explícita:** operações simultâneas, prioridades, fases e efeitos de barramento precisam ter ordenação definida.
3. **Determinismo reproduzível:** mesma configuração, entradas e ROM devem produzir a mesma sequência observável, dentro do contrato definido.
4. **Estado com propriedade clara:** evitar mutação concorrente irrestrita de CPU, barramentos e dispositivos.
5. **Paralelismo seletivo:** paralelizar somente trabalho independente cujo ganho líquido seja medido.
6. **Backpressure:** filas entre subsistemas devem ser limitadas e possuir política explícita para saturação.
7. **Observabilidade:** cada custo relevante deve poder ser medido sem confundir instrumentação com desempenho real.
8. **Compatibilidade de gameplay:** preservar eventos que possam afetar execução, IRQ, leitura de registradores, colisões, áudio ou imagem.
9. **Evolução incremental:** SMS é a primeira máquina; outras entram uma de cada vez.
10. **Legado isolado:** código antigo pode ser estudado, mas não será dependência do V2.

## 3. Separação de tempos

A arquitetura deve distinguir formalmente:

- **Tempo da máquina:** unidades e timestamps virtuais definidos pelo hardware emulado.
- **Tempo de execução do host:** duração real de CPU usada para processar a máquina.
- **Tempo de apresentação:** momento em que áudio/vídeo chegam aos dispositivos físicos.

Esses tempos interagem por contratos, mas não podem ser confundidos. Pacing, WGPU e CPAL não podem alterar retroativamente o tempo virtual.

## 4. Propriedade e concorrência

Hipótese inicial para investigação: um proprietário temporal executa as transições de estado mutável da máquina; workers podem processar tarefas desacopladas, como DSP por blocos, preparação/publicação de vídeo, input capture e tarefas auxiliares.

Essa hipótese não fixa quantidade de threads nem proíbe paralelismo interno. Qualquer divisão da execução de hardware deverá demonstrar independência, sincronização correta, ganho mensurável e ausência de regressões.

## 5. Camadas conceituais

- **Machine orchestration:** ciclo de vida, configuração e comandos coarse-grained.
- **Temporal core:** representação do tempo, agendamento, ordenação e avanço determinístico.
- **Machine implementation:** CPU, barramentos, memória e dispositivos específicos do SMS.
- **Host pipelines:** input, áudio, vídeo e integração com o sistema operacional.
- **Observability and validation:** métricas, testes, replay e benchmarks.

Os nomes e limites finais dos crates serão definidos após o estudo, não presumidos por este documento.

## 6. Critérios de arquitetura aprovada

A arquitetura só será considerada aprovada quando:
- invariantes temporais estiverem escritos e testáveis;
- semântica de eventos simultâneos estiver definida;
- overflow, cancelamento, pausa e retomada tiverem contratos;
- determinismo e replay tiverem definição verificável;
- modelos alternativos tiverem sido comparados;
- custos de dispatch, armazenamento e sincronização tiverem benchmarks representativos;
- revisão independente não encontrar falhas críticas sem resposta.
