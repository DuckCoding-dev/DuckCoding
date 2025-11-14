import { Card, CardContent } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { DndContext, closestCenter } from '@dnd-kit/core';
import type { DragEndEvent, SensorDescriptor, SensorOptions } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { SortableProfileItem } from './SortableProfileItem';
import { ActiveConfigCard } from './ActiveConfigCard';
import type { ToolStatus, ActiveConfig, GlobalConfig } from '@/lib/tauri-commands';

interface ToolProfileTabContentProps {
  tool: ToolStatus;
  profiles: string[];
  activeConfig: ActiveConfig | null;
  globalConfig: GlobalConfig | null;
  transparentProxyEnabled: boolean;
  switching: boolean;
  deletingProfiles: Record<string, boolean>;
  sensors: SensorDescriptor<SensorOptions>[];
  onSwitch: (toolId: string, profile: string) => void;
  onDelete: (toolId: string, profile: string) => void;
  onDragEnd: (event: DragEndEvent) => void;
}

export function ToolProfileTabContent({
  tool,
  profiles,
  activeConfig,
  globalConfig,
  transparentProxyEnabled,
  switching,
  deletingProfiles,
  sensors,
  onSwitch,
  onDelete,
  onDragEnd,
}: ToolProfileTabContentProps) {
  return (
    <Card className="shadow-sm border">
      <CardContent className="pt-6">
        {/* 显示当前生效的配置 */}
        {activeConfig && (
          <ActiveConfigCard
            toolId={tool.id}
            activeConfig={activeConfig}
            globalConfig={globalConfig}
            transparentProxyEnabled={transparentProxyEnabled}
          />
        )}

        {profiles.length > 0 ? (
          <div className="space-y-3">
            <div className="flex items-center gap-2 mb-2">
              <Label>可用的配置文件（拖拽可调整顺序）</Label>
            </div>
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={onDragEnd}
            >
              <SortableContext items={profiles} strategy={verticalListSortingStrategy}>
                <div className="space-y-2">
                  {profiles.map((profile) => (
                    <SortableProfileItem
                      key={profile}
                      profile={profile}
                      toolId={tool.id}
                      switching={switching}
                      deleting={deletingProfiles[`${tool.id}-${profile}`] || false}
                      onSwitch={onSwitch}
                      onDelete={onDelete}
                    />
                  ))}
                </div>
              </SortableContext>
            </DndContext>
          </div>
        ) : (
          <div className="text-center py-8 bg-slate-50 dark:bg-slate-800/50 rounded-lg">
            <p className="text-muted-foreground mb-3">暂无保存的配置文件</p>
            <p className="text-sm text-muted-foreground">
              在"配置 API"页面保存配置时填写名称即可创建多个配置
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
