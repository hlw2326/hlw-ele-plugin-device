import { defineConfig } from 'eslint/config'
import tseslint from '@electron-toolkit/eslint-config-ts'
import stylistic from '@stylistic/eslint-plugin'

export default defineConfig(
    { ignores: ['**/node_modules', '**/dist', 'target/**', 'src/types.d.ts'] },
    tseslint.configs.recommended,
    stylistic.configs.customize({
        indent: 4,
        quotes: 'single',
        semi: false,
        commaDangle: 'never',
        braceStyle: '1tbs',
        jsx: false
    })
)
