/**
 * 自定义时间范围选择对话框
 * 提供分钟级精度的日期时间选择器
 */

import React, { useState, useEffect } from 'react';
import { Calendar as CalendarIcon, Clock } from 'lucide-react';
import { format } from 'date-fns';
import { zhCN } from 'date-fns/locale';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Calendar } from '@/components/ui/calendar';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';
import { validateCustomTimeRange } from '@/utils/time-range';

export interface CustomTimeRangeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  startTime: Date | null;
  endTime: Date | null;
  onStartTimeChange: (date: Date | null) => void;
  onEndTimeChange: (date: Date | null) => void;
  onConfirm: () => void;
}

/**
 * 日期时间选择器组件
 */
function DateTimePicker({
  date,
  onDateChange,
  label,
  maxDate,
}: {
  date: Date | null;
  onDateChange: (date: Date | null) => void;
  label: string;
  maxDate?: Date;
}) {
  const [timeInput, setTimeInput] = useState('00:00');

  useEffect(() => {
    if (date) {
      const hours = date.getHours().toString().padStart(2, '0');
      const minutes = date.getMinutes().toString().padStart(2, '0');
      setTimeInput(`${hours}:${minutes}`);
    }
  }, [date]);

  const handleDateSelect = (selectedDate: Date | undefined) => {
    if (!selectedDate) {
      onDateChange(null);
      return;
    }

    // 保留现有的时间部分
    const [hours, minutes] = timeInput.split(':').map(Number);
    const newDate = new Date(selectedDate);
    newDate.setHours(hours || 0, minutes || 0, 0, 0);
    onDateChange(newDate);
  };

  const handleTimeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setTimeInput(value);

    if (!date) return;

    const [hours, minutes] = value.split(':').map(Number);
    if (isNaN(hours) || isNaN(minutes)) return;

    const newDate = new Date(date);
    newDate.setHours(hours, minutes, 0, 0);
    onDateChange(newDate);
  };

  return (
    <div className="space-y-2">
      <Label>{label}</Label>
      <div className="flex gap-2">
        {/* 日期选择器 */}
        <Popover>
          <PopoverTrigger asChild>
            <Button
              variant="outline"
              className={cn(
                'flex-1 justify-start text-left font-normal',
                !date && 'text-muted-foreground',
              )}
            >
              <CalendarIcon className="mr-2 h-4 w-4" />
              {date ? format(date, 'yyyy-MM-dd', { locale: zhCN }) : '选择日期'}
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-auto p-0" align="start">
            <Calendar
              mode="single"
              selected={date || undefined}
              onSelect={handleDateSelect}
              disabled={(date) => {
                if (maxDate && date > maxDate) return true;
                return false;
              }}
              initialFocus
              locale={zhCN}
            />
          </PopoverContent>
        </Popover>

        {/* 时间输入框 */}
        <div className="relative flex-1">
          <Clock className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            type="time"
            value={timeInput}
            onChange={handleTimeChange}
            className="pl-10"
            disabled={!date}
          />
        </div>
      </div>
    </div>
  );
}

/**
 * 自定义时间范围选择对话框
 */
export function CustomTimeRangeDialog({
  open,
  onOpenChange,
  startTime,
  endTime,
  onStartTimeChange,
  onEndTimeChange,
  onConfirm,
}: CustomTimeRangeDialogProps) {
  const validation = validateCustomTimeRange(startTime, endTime);
  const maxDate = new Date(); // 结束时间不能晚于当前时刻

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>自定义时间范围</DialogTitle>
          <DialogDescription>
            选择起止时间（精确到分钟），时间跨度最多90天，结束时间不能晚于当前时刻
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 开始时间 */}
          <DateTimePicker
            date={startTime}
            onDateChange={onStartTimeChange}
            label="开始时间"
            maxDate={maxDate}
          />

          {/* 结束时间 */}
          <DateTimePicker
            date={endTime}
            onDateChange={onEndTimeChange}
            label="结束时间"
            maxDate={maxDate}
          />

          {/* 错误提示 */}
          {!validation.valid && validation.error && (
            <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {validation.error}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={onConfirm} disabled={!validation.valid}>
            确认
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
