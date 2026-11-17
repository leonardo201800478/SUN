# Controles de áudio e DSP do desktop Slint

## Saída de áudio

O menu **Áudio** oferece volume mestre de 0 a 100%, mute independente, atalhos de 50%/70%, estado da saída e persistência em `%APPDATA%/Sega Emulator/slint_ui_settings.txt`.

Volume e DSP são encaminhados por comandos coarse-grained ao runtime selecionado por `DesktopSession`. O frontend não possui nem manipula diretamente a sessão CPAL. Alterações não reiniciam a máquina, não avançam nem pausam o Scheduler e não modificam registradores ou clocks do PSG.

## Renderer PSG e H2 experimental — somente SMS

O menu de renderização digital é apresentado para SMS1/SMS2:

- **PSG — padrão:** mantém o renderer SN76489 existente.
- **H2 — experimental:** reconstrói a forma de onda a partir de transições observadas no mesmo SN76489 executado pelo Scheduler. Não cria uma segunda instância de PSG nem altera os clocks do hardware.

O SG-1000 não expõe a seleção H2 nem o preset SMS1. Seu perfil digital SN76489 permanece sob controle do runtime SG-1000, sem reaproveitar preferências de renderer específicas do SMS.

## DSP de saída — SMS

Para SMS, o submenu oferece:

- **Preset SMS1 — experimental:** ativa passa-altas de 20 Hz e passa-baixas de 18 kHz.
- **Passa-altas / acoplamento AC:** habilitação independente e ajuste de 5 a 500 Hz.
- **Passa-baixas:** habilitação independente e ajuste de 4 a 20 kHz.
- **Desativar todos os filtros:** retorna ao caminho digital sem processamento analógico adicional.

As preferências SMS existentes (`audio_dsp_preset`, `audio_hpf_*`, `audio_lpf_*`) são preservadas para compatibilidade.

O preset é uma aproximação de saída, não uma reconstrução elétrica calibrada de uma revisão específica. O manual de serviço do Master System indica faixa de saída de 20 Hz a 20 kHz e nível máximo de 300 mVp-p no pino de áudio do IC5; isso não fornece a função de transferência completa, impedância de carga nem a constante RC efetiva da placa. Portanto, 20 Hz e 18 kHz são valores experimentais para avaliação auditiva, não valores medidos de corte dos componentes.

## DSP de saída — SG-1000

Quando SG-1000 está selecionado ou em execução, o menu apresenta **somente** os controles genéricos de DSP de saída:

- passa-altas (HPF), ajustável entre 5 e 500 Hz;
- passa-baixas (LPF), ajustável entre 4 e 20 kHz;
- desativação dos dois filtros.

Não são exibidos o preset SMS1 nem os seletores de renderer PSG/H2. As preferências SG-1000 são independentes das preferências SMS e persistidas em chaves próprias (`sg1000_audio_hpf_enabled`, `sg1000_audio_hpf_cutoff_hz`, `sg1000_audio_lpf_enabled`, `sg1000_audio_lpf_cutoff_hz`). Isso evita que uma alteração de timbre no SG-1000 modifique a saída do SMS, ou vice-versa.

O DSP SG-1000 inicia desativado. Com ambos os filtros desligados, o caminho digital original é preservado. Os filtros são uma opção experimental pós-mixer: a pesquisa atual não dispõe de esquema/captura elétrica SG-1000 suficiente para afirmar uma resposta analógica específica da placa. Não aplicar automaticamente o perfil SMS1 ao SG-1000 nem declarar equivalência elétrica entre as placas.

## Caminho de sinal e invariantes

`PSG → mixer → AnalogOutputProcessor opcional → FIR/resampler → PCM sink`

- O processador é posterior à síntese digital e não altera o perfil do SN76489, o MachineScheduler, CPU, VDP, WAIT states ou clocks.
- HPF e LPF são filtros IIR de primeira ordem, com estado persistente entre blocos.
- Atualizar a configuração altera somente o estado/configuração do DSP; não recria CPAL.
- Pause/Resume preserva a história dos filtros. Reset/Load ROM reinicia o estado DSP; blocos descartados pelo debugger não contaminam o próximo trecho audível.
- A configuração desativada é bypass: não acrescenta filtragem ao caminho original.
- Nenhum filtro é executado por callback de instrução ou por callback do dispositivo de áudio.

## Validação

Testes unitários cobrem validação de frequências, rejeição de DC pelo HPF, bypass exato, perfil experimental e continuidade entre blocos. A separação de preferências SMS/SG-1000 também deve ser coberta por testes de configuração e persistência.

A comparação auditiva com hardware real e a captura de resposta em frequência continuam pendentes; não são declaradas como PASS. Para SG-1000, a habilitação dos filtros não representa uma calibração analógica do console.
