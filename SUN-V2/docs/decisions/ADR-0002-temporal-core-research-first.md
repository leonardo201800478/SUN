# ADR-0002 — Pesquisa antes da implementação do Temporal Core

- **Status:** Aceito
- **Data:** 2026-11-23
- **Escopo:** desenho do núcleo temporal

## Contexto

O Temporal Core será a fundação de CPU, barramentos, memória, VDP, PSG e YM2413. Um erro em sua semântica se propagaria para todos os componentes.

Há diferentes modelos utilizados por emuladores consolidados: timeslices/quanta e timers, corrotinas cooperativas, passos curtos orientados pela CPU e sincronização sob demanda. Nenhum foi demonstrado como universalmente superior para o SUN.

## Decisão

Não iniciar implementação de produção até concluir:
1. pesquisa de documentação primária e projetos de referência;
2. especificação de invariantes e semântica temporal;
3. modelo de referência independente;
4. protótipos comparáveis;
5. benchmarks representativos;
6. revisão adversarial independente.

## Questões explicitamente não decididas

- clock mestre e representação interna do tempo;
- largura inteira e política de overflow;
- clock domains e fase residual;
- fila global, deadlines por dispositivo ou modelo híbrido;
- fases e desempate de eventos no mesmo timestamp;
- granularidade de avanço da CPU e dispositivos;
- API de suspensão/retomada e WAIT states;
- estrutura de dados da agenda;
- política de snapshot/restore.

## Portões de aprovação

- R0 — Pesquisa concluída.
- R1 — Especificação aprovada.
- R2 — Modelo de referência validado.
- R3 — Protótipos e benchmarks comparados.
- R4 — Revisão adversarial concluída.
- R5 — Aprovação formal para implementação.

Falha em qualquer portão retorna o trabalho à etapa correspondente. Não se contorna um portão reduzindo testes ou relaxando invariantes.

## Consequência

A ausência de código de produção nesta fase é intencional. O primeiro resultado técnico deve ser uma especificação verificável, não um scheduler prematuro.
