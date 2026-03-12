import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useTheme } from '@/hooks/useThemeHook';
import { hexToHslToken, hslTokenToHex, ThemeTokens } from '@/hooks/theme-palette';

interface ThemeCustomizerDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const THEME_FIELDS: Array<{ key: keyof ThemeTokens; label: string; description: string }> = [
  { key: 'background', label: '背景', description: '页面主背景色' },
  { key: 'foreground', label: '正文', description: '默认文本颜色' },
  { key: 'card', label: '卡片', description: '卡片和浮层基础底色' },
  { key: 'cardForeground', label: '卡片文字', description: '卡片内文本颜色' },
  { key: 'primary', label: '主色', description: '按钮、选中、高亮' },
  { key: 'primaryForeground', label: '主色文字', description: '主色按钮上的文字颜色' },
  { key: 'muted', label: '弱化底色', description: '弱化容器和占位块' },
  { key: 'mutedForeground', label: '弱化文字', description: '次要说明文字' },
  { key: 'accent', label: '强调底色', description: '悬浮和辅助高亮' },
  { key: 'accentForeground', label: '强调文字', description: '强调区域中的文字颜色' },
  { key: 'border', label: '边框', description: '分割线和边框色' },
  { key: 'input', label: '输入框', description: '输入边框和输入底色' },
  { key: 'ring', label: '焦点环', description: '聚焦状态的高亮色' },
];

function ThemeSection({ mode, title }: { mode: 'light' | 'dark'; title: string }) {
  const { palette, updatePaletteToken } = useTheme();
  const tokens = palette[mode];

  return (
    <section className="space-y-4 rounded-xl border border-border/70 bg-card/70 p-4">
      <div>
        <h3 className="text-sm font-semibold text-foreground">{title}</h3>
        <p className="mt-1 text-xs text-muted-foreground">修改后会立即应用到当前主题系统。</p>
      </div>

      <div className="space-y-3">
        {THEME_FIELDS.map((field) => (
          <div
            key={`${mode}-${field.key}`}
            className="grid grid-cols-[120px_1fr_auto] items-center gap-3 rounded-lg border border-border/60 bg-background/70 p-3"
          >
            <div>
              <Label className="text-sm font-medium text-foreground">{field.label}</Label>
              <p className="mt-1 text-[11px] leading-4 text-muted-foreground">
                {field.description}
              </p>
            </div>

            <Input
              value={tokens[field.key]}
              onChange={(event) => updatePaletteToken(mode, field.key, event.target.value)}
              className="font-mono text-xs"
            />

            <Input
              type="color"
              value={hslTokenToHex(tokens[field.key])}
              onChange={(event) =>
                updatePaletteToken(mode, field.key, hexToHslToken(event.target.value))
              }
              className="h-10 w-14 cursor-pointer rounded-lg border border-border bg-background p-1"
              aria-label={`${title}${field.label}`}
            />
          </div>
        ))}
      </div>
    </section>
  );
}

export function ThemeCustomizerDialog({ open, onOpenChange }: ThemeCustomizerDialogProps) {
  const { actualTheme, hasCustomPalette, palette, preset, setPreset, resetPalette } = useTheme();
  const preview = palette[actualTheme];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-5xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>自定义主题</DialogTitle>
          <DialogDescription>
            显示模式仍由浅色、深色、系统控制。这里编辑的是配色预设和浅深两套调色盘。
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6">
          <section
            className="rounded-2xl border border-border/70 p-5"
            style={{
              background: `linear-gradient(135deg, hsl(${preview.background}), hsl(${preview.accent}))`,
              color: `hsl(${preview.foreground})`,
            }}
          >
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div>
                <p className="text-xs uppercase tracking-[0.2em] opacity-70">Theme Preview</p>
                <h3 className="mt-2 text-xl font-semibold">
                  当前为 {actualTheme === 'dark' ? '深色' : '浅色'} 预览
                </h3>
                <p className="mt-2 max-w-xl text-sm opacity-80">
                  当前配色方案：
                  {preset === 'default' ? ' 默认' : preset === 'green' ? ' 绿色' : ' 自定义'}
                </p>
              </div>

              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  variant={preset === 'default' ? 'default' : 'secondary'}
                  onClick={() => setPreset('default')}
                >
                  使用默认
                </Button>
                <Button
                  type="button"
                  variant={preset === 'green' ? 'default' : 'secondary'}
                  onClick={() => setPreset('green')}
                >
                  使用绿色
                </Button>
                {hasCustomPalette && (
                  <Button
                    type="button"
                    variant={preset === 'custom' ? 'default' : 'secondary'}
                    onClick={() => setPreset('custom')}
                  >
                    使用自定义
                  </Button>
                )}
                <Button type="button" variant="outline" onClick={resetPalette}>
                  恢复默认
                </Button>
              </div>
            </div>

            <div className="mt-5 grid gap-3 md:grid-cols-3">
              <div
                className="rounded-xl border border-black/10 p-4 shadow-sm"
                style={{
                  backgroundColor: `hsl(${preview.card})`,
                  color: `hsl(${preview.cardForeground})`,
                }}
              >
                <p className="text-sm font-semibold">卡片预览</p>
                <p className="mt-2 text-xs opacity-80">卡片、面板和多数容器会跟随这里的 token。</p>
              </div>
              <div
                className="rounded-xl border border-black/10 p-4 shadow-sm"
                style={{
                  backgroundColor: `hsl(${preview.primary})`,
                  color: `hsl(${preview.primaryForeground})`,
                }}
              >
                <p className="text-sm font-semibold">主按钮</p>
                <p className="mt-2 text-xs opacity-90">主按钮、选中导航和焦点色会使用这组颜色。</p>
              </div>
              <div
                className="rounded-xl border border-black/10 p-4 shadow-sm"
                style={{
                  backgroundColor: `hsl(${preview.muted})`,
                  color: `hsl(${preview.mutedForeground})`,
                }}
              >
                <p className="text-sm font-semibold">弱化区域</p>
                <p className="mt-2 text-xs opacity-90">
                  说明文字、浅背景和辅助区域会跟随这组颜色。
                </p>
              </div>
            </div>
          </section>

          <div className="grid gap-4 xl:grid-cols-2">
            <ThemeSection mode="light" title="浅色调色盘" />
            <ThemeSection mode="dark" title="深色调色盘" />
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
