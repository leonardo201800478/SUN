# Perfil de sistema ativo — Desktop SMS

## Sistema ativo

O host desktop executa atualmente o **Sega Master System I**, usando o ASIC VDP **Sega 315-5124** (SmsVdpRevision::Sega3155124).

A seleção regional é independente da revisão do ASIC:

- USA: NTSC
- Brasil: PAL-M
- Europa: PAL
- Japão: NTSC

A função sms_temporal_region_profile do runtime seleciona a revisão SMS1 para todas as quatro regiões e varia o padrão de vídeo/clock conforme a região.

## Perfis distintos

- **SMS1 / Sega 315-5124:** perfil ativo no desktop.
- **SMS2 / Sega 315-5246:** revisão distinta existente no modelo de perfis do VDP, mas não selecionada nem liberada pelo runtime desktop.
- **SG-1000 / TMS9918:** hardware separado; não deve reutilizar o VDP SMS1.
- **Game Gear, Mega Drive, Sega CD e 32X:** não habilitados neste host.

A interface mostra o SMS1 como sistema ativo e mantém os demais perfis explicitamente indisponíveis. Não existe fallback automático entre revisões de VDP.

## Critério para habilitar outro perfil

Um perfil futuro só poderá ser habilitado quando possuir seleção explícita no runtime, configuração regional/temporal própria quando aplicável, integração de barramento e testes de regressão direcionados. A existência de uma enumeração de ASIC não equivale à compatibilidade funcional do sistema completo.
