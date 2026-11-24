# ADR-0003 — construir o SMS V2 por uma fatia vertical mínima

- **Status:** Aceito como estratégia de desenvolvimento; detalhes de implementação continuam sujeitos aos gates R0–R5.
- **Data:** 2026-11-24.
- **Contexto:** SUN V2 / CHRONOS.
- **Decisores:** arquitetura do SUN V2.

## Contexto

A V1 já possui componentes funcionais relevantes para SMS: executor Z80, barramento/memória/cartucho, VDP SMS, PSG SN76489, runtime de máquina e infraestrutura de áudio. Entretanto, a existência desses componentes não significa que todos tenham a mesma granularidade temporal, nem que a integração esteja livre de lacunas.

As auditorias disponíveis registram, entre outros pontos, execução Z80 por instrução completa com T-states agregados no contrato documentado do crate compartilhado, VDP com renderização por scanline e refinamentos seletivos de fronteira, PSG digital com clock/divisor e geração de amostras, e YM2413 ainda dependente de decisões específicas de placa, clock, roteamento e integração. Essas informações são fotografias de commits e escopos identificados; cada referência precisa ser revalidada no SHA atual antes de servir como fato de implementação.

O SUN V2, por decisão de projeto, é uma reconstrução independente dentro de `SUN-V2/`. O código V1 não será dependência nem será copiado automaticamente.

## Decisão

Construir o V2 por **fatias verticais mínimas e mensuráveis**, integrando CPU, barramento/memória, VDP e PSG desde o primeiro protótipo de máquina.

A primeira fatia não busca compatibilidade ampla. Busca uma sequência reproduzível que exercite causalidade entre CPU, barramento, VDP e áudio, permita medir custo individual e revele cedo problemas de contrato.

A sequência de preparação é:
1. inventário crítico V1→V2 e fechamento de evidências (R0);
2. contratos temporais e de barramento mais modelo de referência independente (R1–R2);
3. protótipo da máquina mínima integrada (R3);
4. comparação de alternativas e revisão adversarial (R4);
5. aprovação arquitetural antes de produção (R5).

## Regras

1. Componentes são mínimos em escopo funcional, mas sólidos em contratos, determinismo e ownership.
2. Não construir CPU, VDP e áudio como três protótipos sem integração temporal até o final.
3. O barramento é uma fronteira explícita entre CPU, memória, I/O e dispositivos.
4. O CHRONOS mantém autoridade temporal; cada dispositivo mantém seu estado.
5. Microsequenciamento só é introduzido nos pontos em que o hardware/jogo/teste exige observação temporal intermediária.
6. A semântica funcional suportada deve ser correta; subconjuntos são explicitamente delimitados e nunca fingem compatibilidade ampla.
7. PSG é a primeira fonte de áudio; YM2413 é uma etapa posterior, condicionada a evidências de hardware e perfil.
8. Testes V1 podem orientar o V2 somente com proveniência, licença, independência e revalidação.
9. Cada fatia mede custo, alocações, eventos, sincronizações e determinismo.
10. Código de produção não começa antes da aprovação R5; protótipos anteriores são descartáveis e ficam claramente separados.

## Consequências positivas

- Problemas de integração aparecem cedo.
- Custo e precisão podem ser atribuídos a cada subsistema.
- A V1 reduz a incerteza sobre comportamento funcional sem contaminar a arquitetura V2.
- O projeto evita construir três engines isoladas com contratos incompatíveis.
- A precisão temporal pode crescer em resposta a evidência e não por uniformidade artificial.

## Consequências negativas e riscos

- A primeira fatia exige mais coordenação do que três protótipos isolados.
- Um subconjunto mínimo não executará imediatamente muitos jogos.
- Contratos prematuros podem precisar de revisão quando surgirem evidências de hardware.
- A separação entre protótipo e produção precisa ser rigorosa para evitar que código experimental vire dependência acidental.

## Alternativas rejeitadas

### A. Reimplementar CPU inteira antes de VDP e áudio

Rejeitada porque adia a validação da integração temporal e pode otimizar um executor com contrato de barramento inadequado.

### B. Criar CPU, VDP e áudio isolados e integrá-los somente no final

Rejeitada porque mascara dependências de timestamp, fase, IRQ, portas e continuidade de áudio.

### C. Copiar os componentes V1 e refatorá-los depois

Rejeitada porque transforma decisões e limitações temporais herdadas em premissas estruturais do V2, contrariando a independência aprovada.

### D. Implementar toda a precisão temporal antes de qualquer dispositivo funcional

Rejeitada porque o modelo temporal precisa de cenários concretos e observáveis. O CHRONOS será especificado primeiro, mas validado por uma máquina mínima integrada.

## Critérios de revisão desta decisão

Revisar este ADR se:
- a primeira fatia mostrar que uma das integrações não é necessária para testar a hipótese temporal;
- o modelo de referência exigir granularidade diferente;
- benchmarks demonstrarem que a composição proposta cria custo injustificado;
- evidência de hardware alterar a ordem causal ou o ownership;
- o escopo mínimo não produzir cenário discriminante útil.
