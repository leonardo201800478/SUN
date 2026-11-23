# temporal_bench

Harness de latência e alocação do `MachineScheduler`, criado para atender ao §4.7 do
[plano de otimização](../../docs/plans/temporal-core-optimization-plan.md): medir o
dispatch na **ordem de produção**, com o tipo real `ClockAdvance`, contabilizando
todas as alocações.

## Por que existe

A primeira auditoria comparou uma janela instrumentada em volta de `dispatch_next`
com um microbenchmark que usava um tipo menor e pulava parte do trabalho. As duas
fronteiras não eram comparáveis. Este harness fixa uma única fronteira: tudo o que
um dispatch real faz, na ordem em que faz.

## Como rodar

```sh
cd tools/temporal_bench
cargo run --release --bin temporal_bench   # caminho com Vec (histórico)
cargo run --release --bin caller_buffer    # caminho sem alocação
```

`temporal_bench` compila também contra o scheduler anterior à P0, para que o A/B
tenha os dois lados na mesma fronteira.

## O que está incluído na região medida

`next_timestamp()`, orçamento por timestamp, conversão racional por domínio,
validação e commit dos avanços, `pop_next`, `trace!` de dispatch e construção do
`MachineDispatch`. Excluído: o agendamento do evento (medido no mesmo laço) e o
trabalho dos consumidores (VDP, PSG, `sms_core`).
