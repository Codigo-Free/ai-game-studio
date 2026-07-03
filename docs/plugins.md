# Plugins

Toda funcionalidad de AI Game Studio podrá extenderse mediante plugins y SDKs (principio **Extensible**). El ecosistema de plugins corresponde a la **Fase 5**, pero la arquitectura se prepara desde el principio.

---

## SDK

El SDK (`sdk/`) es el contrato público del sistema:

- Especificación del formato de proyecto `.aigs` (disponible desde el MVP).
- APIs para extender el editor: paneles, herramientas, importadores de assets.
- APIs para extender el runtime: componentes y sistemas personalizados.
- APIs para agentes IA de comunidad (sobre el AI Core, ver [ia.md](ia.md)).

## Tipos de extensiones previstas

- Plugins de editor (paneles, herramientas).
- Componentes y sistemas del motor.
- Exportadores de plataforma.
- Importadores de formatos de assets.
- Plantillas de proyecto y paquetes de contenido.
- Agentes IA especializados.

## Marketplace (Fase 5)

Distribución, descubrimiento, versionado e instalación de plugins desde el propio editor, con publicación abierta a la comunidad.

## Preparación durante el MVP

- Paneles del editor como módulos independientes sobre el sistema de docking.
- Componentes del runtime registrados en un catálogo (no hardcodeados en el loop).
- El formato `.aigs` admite componentes de origen externo con namespace propio.
