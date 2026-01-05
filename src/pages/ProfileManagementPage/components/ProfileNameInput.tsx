/**
 * Profile 名称输入组件
 *
 * 共享的 Profile 名称输入框，提供验证提示
 */

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface ProfileNameInputProps {
  /** 输入值 */
  value: string;
  /** 值变更回调 */
  onChange: (value: string) => void;
  /** 占位符文本 */
  placeholder?: string;
}

/**
 * Profile 名称输入组件
 */
export function ProfileNameInput({ value, onChange, placeholder }: ProfileNameInputProps) {
  return (
    <div className="space-y-2">
      <Label htmlFor="profile-name">Profile 名称 *</Label>
      <Input
        id="profile-name"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder || '例如: my_profile'}
        required
      />
      <p className="text-xs text-muted-foreground">
        为 Profile 设置一个本地名称（不能以 dc_proxy_ 开头）
      </p>
    </div>
  );
}
