import { useCallback } from 'react';
import { useSensor, useSensors, PointerSensor, KeyboardSensor, DragEndEvent } from '@dnd-kit/core';
import { arrayMove, sortableKeyboardCoordinates } from '@dnd-kit/sortable';

/**
 * Hook for managing profile drag-and-drop sorting
 */
export function useProfileSorting() {
  // 拖拽传感器
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  // 保存配置文件排序到localStorage
  const saveProfileOrder = useCallback((toolId: string, order: string[]) => {
    try {
      const key = `profile-order-${toolId}`;
      localStorage.setItem(key, JSON.stringify(order));
    } catch (error) {
      console.error('Failed to save profile order:', error);
    }
  }, []);

  // 应用已保存的排序
  const applySavedOrder = useCallback((toolId: string, profiles: string[]): string[] => {
    let savedOrder: string[] = [];
    try {
      const key = `profile-order-${toolId}`;
      const saved = localStorage.getItem(key);
      savedOrder = saved ? JSON.parse(saved) : [];
    } catch (error) {
      console.error('Failed to load profile order:', error);
    }

    if (savedOrder.length === 0) return profiles;

    // 按照保存的顺序排列
    const ordered: string[] = [];
    const remaining = [...profiles];

    savedOrder.forEach((name) => {
      const index = remaining.indexOf(name);
      if (index !== -1) {
        ordered.push(name);
        remaining.splice(index, 1);
      }
    });

    // 将新增的配置文件添加到末尾
    return [...ordered, ...remaining];
  }, []);

  // 处理拖拽结束事件
  const createDragEndHandler = useCallback(
    (toolId: string, setProfiles: React.Dispatch<React.SetStateAction<Record<string, string[]>>>) =>
      (event: DragEndEvent) => {
        const { active, over } = event;

        if (over && active.id !== over.id) {
          setProfiles((prevProfiles) => {
            const toolProfiles = prevProfiles[toolId] || [];
            const oldIndex = toolProfiles.indexOf(active.id as string);
            const newIndex = toolProfiles.indexOf(over.id as string);

            if (oldIndex === -1 || newIndex === -1) return prevProfiles;

            const newOrder = arrayMove(toolProfiles, oldIndex, newIndex);
            saveProfileOrder(toolId, newOrder);

            return {
              ...prevProfiles,
              [toolId]: newOrder,
            };
          });
        }
      },
    [saveProfileOrder],
  );

  return {
    sensors,
    saveProfileOrder,
    applySavedOrder,
    createDragEndHandler,
  };
}
