# Cross-Lab Themes

Cross-Lab themes are renderer-neutral presentation data governed by ADR-0012. Native renderers map this contract into GPUI/GPUI Kit, Android Kotlin/Compose, and later Swift types. Theme data never carries trust, policy, capability, network-classification, or privilege authority.

## Contract

`design/schema/theme-v1.schema.json` defines schema version `1`. Colors are stored as structured OKLCH values (`l`, `c`, `h`, optional `alpha`) rather than CSS strings. Numeric layout, typography, spacing, radius, control, icon, shadow, and motion values are logical design units; each renderer maps them using its platform-native density/accessibility rules without changing the semantic role.

The baseline remains sharp geometry. Current baseline themes therefore use radius `0` for the consumed geometry tokens, while the schema keeps radius as data so a future reviewed theme can intentionally choose different geometry without feature-code branching.

## Ayu Light provenance

`ayu-light.json` is a Cross-Lab semantic translation of the Ayu Light reference shipped by GPUI Kit `0.6.1` at commit `36b51819deb52c947a79f8de29e0e9175eda7464`, file `themes/ayu.json`. GPUI Kit `0.6.1` is distributed under Apache-2.0 and the referenced theme file identifies Zed Industries as its author and links its upstream Ayu source at Zed commit `e62dd2a0e584154886ff86cc1e9e3e060558b977`.

Only the semantic values needed by M10 are translated. Cross-Lab does not copy GPUI Kit's component-specific theme schema or make GPUI Kit types part of the shared design contract. The source sRGB palette is adapted into structural OKLCH values for native renderer consumption; typography, spacing, geometry, control metrics, elevation, and motion are Cross-Lab baseline tokens rather than claims about Ayu upstream defaults.

## Darkmatter gate

`Darkmatter` is the default dark theme identity, but its authoritative owner-provided palette is not present in the verified project sources. Do not create `darkmatter.json` from screenshots, approximation, or a substitute theme. Until the authoritative source is supplied, dark-theme resolution must surface a typed unavailable/configuration state rather than silently fabricating product values.

## Fixtures

`design/fixtures/theme-v1-valid.json` is a synthetic contract fixture for parser/renderer tests. `theme-v1-invalid.json` is intentionally schema-invalid while remaining syntactically valid JSON, so consumers can prove fail-closed parsing without relying on malformed JSON.
