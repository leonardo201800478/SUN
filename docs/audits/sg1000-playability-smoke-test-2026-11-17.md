# SG-1000 — registro de jogabilidade (smoke test manual)

**Data do relato:** 2026-11-17  
**Projeto:** Sega Emulator  
**Baseline remota atual no início da tarefa:** `main @ 0a7b24fe014d11ec7adbc13c1e7ca79a68d0a315`  
**Tipo de evidência:** teste manual reportado pelo usuário no desktop local

## Resultado

**Status: JOGÁVEL — validação prática inicial.**

O usuário informou que, após atualizar a cópia local da `main`, executou jogos SG-1000 e observou:

| Área | Resultado reportado |
|---|---|
| Execução dos jogos | Velocidade normal; jogabilidade funcional |
| Vídeo | Gráficos apresentados corretamente |
| Áudio | Som reproduzido corretamente |
| Entrada | Controles funcionando |
| Cartuchos com expansão de RAM | ROMs testadas com expansão de RAM funcionando |

A interface foi considerada visualmente insatisfatória pelo usuário. Seu refinamento foi explicitamente adiado e não faz parte deste registro de validação do runtime.

## Limites da evidência

- Os nomes, hashes e versões das ROMs não foram fornecidos; portanto, não é possível associar o relato a perfis individuais de cartucho.
- O resultado cobre os jogos efetivamente testados pelo usuário, não todos os títulos do catálogo SG-1000.
- Não representa certificação exaustiva de todas as configurações de RAM, variantes de placa, padrões de vídeo ou sequências de entrada.
- Não foram fornecidos logs de pacing, contadores de underrun, capturas de áudio/vídeo ou comparação com console físico.
- O relato confirma funcionamento prático de vídeo, áudio, controles e expansão de RAM nos casos testados; não substitui testes automatizados ou validação elétrica.

## Preservação arquitetural

O registro não autoriza mudanças no núcleo SN76489, Z80, MachineScheduler, VDP ou barramentos. Melhorias futuras devem permanecer na camada de apresentação/integração, salvo defeito reproduzível com evidência técnica.

## Próximo refinamento de áudio

A interface deverá expor para SG-1000 apenas HPF/LPF opcionais de saída, com preferências independentes das opções SMS. O bypass digital permanece padrão. Como não há esquema/captura elétrica SG-1000 suficiente para caracterizar a resposta da placa, esses filtros devem ser identificados como DSP experimental, nunca como perfil analógico calibrado.

