import { useState } from 'react';
import {
  DndContext,
  DragEndEvent,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import {
  SortableContext,
  arrayMove,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { GripVertical, Plus, Trash2 } from 'lucide-react';
import { SecretInput } from '@/components/SecretInput';
import { cn } from '@/lib/utils';
import type { JsonObject, JsonValue } from '@/lib/tauri-commands';
import {
  CUSTOM_FIELD_TYPE_OPTIONS,
  DEFAULT_DESCRIPTION,
  type CustomFieldType,
  type JSONSchema,
  type SchemaFieldProps,
} from './types';
import {
  getDefaultValue,
  getEffectiveType,
  isCompoundField,
  isJsonObject,
  resolveSchema,
  formatPath,
  getObjectFromPath,
  createSchemaForType,
  getTypeLabel,
  safeStringify,
  isJsonSchemaObject,
} from './utils';

export function SchemaField({
  schema,
  value,
  onChange,
  onDelete,
  path,
  rootSchema,
  isRequired,
  showDescription = true,
  inline = false,
  rootValue,
}: SchemaFieldProps) {
  const resolvedSchema = resolveSchema(schema, rootSchema);
  const description = resolvedSchema?.description ?? DEFAULT_DESCRIPTION;
  const effectiveType = getEffectiveType(resolvedSchema, value);

  if (effectiveType === 'object') {
    return (
      <ObjectField
        schema={resolvedSchema ?? { type: 'object', additionalProperties: true }}
        value={value}
        onChange={onChange}
        path={path}
        rootSchema={rootSchema}
        description={description}
        rootValue={rootValue}
      />
    );
  }

  if (effectiveType === 'array') {
    return (
      <ArrayField
        schema={resolvedSchema ?? { type: 'array', items: {} }}
        value={value}
        onChange={onChange}
        path={path}
        rootSchema={rootSchema}
        description={description}
        rootValue={rootValue}
      />
    );
  }

  if (effectiveType === 'boolean') {
    return (
      <BooleanField
        value={value}
        onChange={onChange}
        description={description}
        showDescription={showDescription}
        inline={inline}
      />
    );
  }

  if (effectiveType === 'number' || effectiveType === 'integer') {
    return (
      <NumberField
        value={value}
        onChange={onChange}
        description={description}
        showDescription={showDescription}
        inline={inline}
      />
    );
  }

  if (effectiveType === 'string') {
    return (
      <StringField
        schema={resolvedSchema}
        value={value}
        onChange={onChange}
        description={description}
        showDescription={showDescription}
        inline={inline}
        rootValue={rootValue}
      />
    );
  }

  return (
    <FallbackJsonField
      value={value}
      onChange={onChange}
      description={description}
      allowDelete={!isRequired}
      onDelete={onDelete}
      showDescription={showDescription}
      inline={inline}
    />
  );
}

function StringField({
  schema,
  value,
  onChange,
  description,
  showDescription = true,
  inline = false,
  rootValue,
}: {
  schema?: JSONSchema;
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  description: string;
  showDescription?: boolean;
  inline?: boolean;
  rootValue: JsonObject;
}) {
  const currentValue = typeof value === 'string' ? value : '';
  const enumValues = schema?.enum?.map((item) => String(item)) ?? [];
  const derivedOptions =
    schema?.['x-key-source'] && isJsonObject(rootValue)
      ? Object.keys(getObjectFromPath(rootValue, schema['x-key-source'] as string) ?? {})
      : [];
  const selectOptions = enumValues.length > 0 ? enumValues : derivedOptions;
  const hasSelectOptions = selectOptions.length > 0;
  const matchedOption =
    hasSelectOptions && selectOptions.includes(currentValue) ? currentValue : undefined;
  const CUSTOM_OPTION_VALUE = '__custom__';
  const isCustomSelected = hasSelectOptions ? !matchedOption : false;
  const shouldShowInput = !hasSelectOptions || isCustomSelected;
  const isSecretField = Boolean(schema?.['x-secret']);
  const renderTextInput = (inputClassName: string, parentIsRelative = false) => {
    if (isSecretField) {
      return (
        <SecretInput
          className={inputClassName}
          value={currentValue}
          onValueChange={(next) => onChange(next)}
          placeholder="请输入自定义内容"
          toggleLabel={`切换${schema?.title ?? '字段'}可见性`}
          withWrapper={!parentIsRelative}
          wrapperClassName={parentIsRelative ? undefined : 'w-full'}
        />
      );
    }

    return (
      <Input
        className={inputClassName}
        value={currentValue}
        onChange={(event) => onChange(event.target.value)}
        placeholder="请输入自定义内容"
      />
    );
  };

  const renderSelect = (triggerClass: string) => (
    <Select
      value={isCustomSelected ? CUSTOM_OPTION_VALUE : (matchedOption ?? selectOptions[0])}
      onValueChange={(val) => {
        if (val === CUSTOM_OPTION_VALUE) {
          if (!isCustomSelected) {
            onChange('');
          }
          return;
        }
        onChange(val);
      }}
    >
      <SelectTrigger className={triggerClass}>
        <SelectValue placeholder="选择选项" />
      </SelectTrigger>
      <SelectContent>
        {selectOptions.map((option) => (
          <SelectItem key={option} value={option}>
            {option}
          </SelectItem>
        ))}
        <SelectItem value={CUSTOM_OPTION_VALUE}>自定义</SelectItem>
      </SelectContent>
    </Select>
  );

  if (inline) {
    const selectClass = isCustomSelected ? 'w-fit min-w-[140px]' : 'flex-1 min-w-0';
    const inlineContainerClass = cn('flex w-full items-center gap-3 min-w-0', {
      relative: isSecretField,
    });
    return (
      <div className={inlineContainerClass}>
        {hasSelectOptions && renderSelect(selectClass)}
        {shouldShowInput && renderTextInput('flex-1 min-w-0', isSecretField)}
      </div>
    );
  }

  return (
    <div className="space-y-1">
      <div
        className={cn('flex flex-wrap items-center gap-3', {
          relative: isSecretField,
        })}
      >
        {hasSelectOptions &&
          renderSelect(isCustomSelected ? 'w-fit min-w-[140px]' : 'flex-1 min-w-[200px]')}
        {shouldShowInput && renderTextInput('flex-1 min-w-[200px]', isSecretField)}
      </div>
      {showDescription && <p className="text-xs text-muted-foreground">{description}</p>}
    </div>
  );
}

function NumberField({
  value,
  onChange,
  description,
  showDescription = true,
  inline = false,
}: {
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  description: string;
  showDescription?: boolean;
  inline?: boolean;
}) {
  const currentValue = typeof value === 'number' && Number.isFinite(value) ? value : '';

  if (inline) {
    return (
      <div className="flex w-full items-center justify-end gap-3 min-w-0">
        <Input
          className="w-48"
          type="number"
          value={currentValue}
          onChange={(event) => onChange(event.target.value === '' ? 0 : Number(event.target.value))}
          placeholder="请输入数字"
        />
      </div>
    );
  }

  return (
    <div className="space-y-1">
      <div className="flex flex-wrap items-center gap-3">
        <Input
          className="w-40"
          type="number"
          value={currentValue}
          onChange={(event) => onChange(event.target.value === '' ? 0 : Number(event.target.value))}
          placeholder="请输入数字"
        />
      </div>
      {showDescription && <p className="text-xs text-muted-foreground">{description}</p>}
    </div>
  );
}

function BooleanField({
  value,
  onChange,
  description,
  showDescription = true,
  inline = false,
}: {
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  description: string;
  showDescription?: boolean;
  inline?: boolean;
}) {
  const currentValue = typeof value === 'boolean' ? value : false;

  if (inline) {
    return (
      <div className="flex w-full items-center justify-end gap-3 min-w-0">
        <Switch checked={currentValue} onCheckedChange={(checked) => onChange(checked)} />
        <span className="text-sm font-medium">{currentValue ? '启用' : '禁用'}</span>
      </div>
    );
  }

  return (
    <div className="space-y-1">
      <div className="flex flex-wrap items-center gap-3">
        <Switch checked={currentValue} onCheckedChange={(checked) => onChange(checked)} />
        <span className="text-sm font-medium">{currentValue ? '启用' : '禁用'}</span>
      </div>
      {showDescription && <p className="text-xs text-muted-foreground">{description}</p>}
    </div>
  );
}

function ArrayField({
  schema,
  value,
  onChange,
  path,
  rootSchema,
  description,
  rootValue,
}: {
  schema: JSONSchema;
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  path: (string | number)[];
  rootSchema: JSONSchema | null;
  description: string;
  rootValue: JsonObject;
}) {
  const itemsSchema =
    Array.isArray(schema.items) && schema.items.length > 0
      ? schema.items[0]
      : (schema.items as JSONSchema | undefined);
  const resolvedItemsSchema = resolveSchema(itemsSchema, rootSchema);
  const currentValue = Array.isArray(value) ? value : [];
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));
  const itemIds: string[] = currentValue.map((_, index) => formatPath([...path, index]));
  const shouldShowDescription =
    description && description !== DEFAULT_DESCRIPTION && description.trim().length > 0;

  const handleItemChange = (index: number, newValue: JsonValue) => {
    const next = [...currentValue];
    next[index] = newValue;
    onChange(next);
  };

  const handleRemoveItem = (index: number) => {
    const next = currentValue.filter((_, idx) => idx !== index);
    onChange(next);
  };

  const handleAddItem = () => {
    const next = [...currentValue, getDefaultValue(resolvedItemsSchema)];
    onChange(next);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) {
      return;
    }
    const oldIndex = itemIds.indexOf(active.id as string);
    const newIndex = itemIds.indexOf(over.id as string);
    if (oldIndex === -1 || newIndex === -1) {
      return;
    }
    onChange(arrayMove(currentValue, oldIndex, newIndex));
  };

  return (
    <div className="space-y-3">
      {shouldShowDescription && <p className="text-xs text-muted-foreground">{description}</p>}
      {currentValue.length === 0 ? (
        <div className="rounded-md border border-dashed p-4 text-center text-xs text-muted-foreground">
          当前数组为空
        </div>
      ) : (
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
          <SortableContext items={itemIds} strategy={verticalListSortingStrategy}>
            <div className="space-y-2">
              {currentValue.map((item, index) => (
                <SortableArrayItem
                  key={itemIds[index]}
                  id={itemIds[index]}
                  schema={resolvedItemsSchema}
                  value={item}
                  onChange={(newValue) => handleItemChange(index, newValue)}
                  onDelete={() => handleRemoveItem(index)}
                  path={[...path, index]}
                  rootSchema={rootSchema}
                  rootValue={rootValue}
                />
              ))}
            </div>
          </SortableContext>
        </DndContext>
      )}
      <Button variant="outline" size="sm" onClick={handleAddItem}>
        <Plus className="mr-1.5 h-4 w-4" />
        新增项目
      </Button>
    </div>
  );
}

interface SortableArrayItemProps {
  id: string;
  schema?: JSONSchema;
  value: JsonValue;
  onChange: (value: JsonValue) => void;
  onDelete: () => void;
  path: (string | number)[];
  rootSchema: JSONSchema | null;
  rootValue: JsonObject;
}

function SortableArrayItem({
  id,
  schema,
  value,
  onChange,
  onDelete,
  path,
  rootSchema,
  rootValue,
}: SortableArrayItemProps) {
  const resolvedSchema = resolveSchema(schema, rootSchema);
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`flex items-center gap-3 rounded-md border bg-card px-3 py-2 ${
        isDragging ? 'opacity-70' : ''
      }`}
    >
      <button
        type="button"
        {...attributes}
        {...listeners}
        className="cursor-grab text-slate-400 hover:text-slate-600 focus-visible:outline-none"
        aria-label="拖拽排序"
      >
        <GripVertical className="h-4 w-4" />
      </button>
      <div className="flex-1 min-w-0">
        <SchemaField
          schema={resolvedSchema}
          value={value}
          onChange={onChange}
          path={path}
          rootSchema={rootSchema}
          inline={!isCompoundField(resolvedSchema, value)}
          showDescription={false}
          rootValue={rootValue}
        />
      </div>
      <Button variant="ghost" size="sm" onClick={onDelete}>
        <Trash2 className="h-4 w-4 text-destructive" />
      </Button>
    </div>
  );
}

function ObjectField({
  schema,
  value,
  onChange,
  path,
  rootSchema,
  description,
  rootValue,
}: {
  schema: JSONSchema;
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  path: (string | number)[];
  rootSchema: JSONSchema | null;
  description: string;
  rootValue: JsonObject;
}) {
  const objectValue = isJsonObject(value) ? value : {};
  const requiredKeys = schema.required ?? [];
  const currentKeys = Object.keys(objectValue);
  const keys = Array.from(new Set([...currentKeys, ...requiredKeys]));
  const schemaDefinedKeys = schema.properties ? Object.keys(schema.properties) : [];
  const availableSchemaKeys = schemaDefinedKeys
    .filter((key) => !currentKeys.includes(key) && !requiredKeys.includes(key))
    .sort((a, b) => a.localeCompare(b));
  const availableSchemaOptions = availableSchemaKeys.map((optionKey) => {
    const optionSchema = resolveObjectChildSchema(schema, optionKey, rootSchema);
    return {
      key: optionKey,
      description: optionSchema?.description ?? DEFAULT_DESCRIPTION,
    };
  });
  const isEnvObject = path.length === 1 && path[0] === 'env';
  const canAddCustomField = schema.additionalProperties !== false || !!schema.patternProperties;
  const [customKey, setCustomKey] = useState('');
  const [customFieldType, setCustomFieldType] = useState<CustomFieldType>('string');
  const [schemaKeyToAdd, setSchemaKeyToAdd] = useState('');

  const handleChildChange = (key: string, newValue: JsonValue) => {
    const next = { ...objectValue, [key]: newValue };
    onChange(next);
  };

  const handleDeleteChild = (key: string) => {
    const next = { ...objectValue };
    delete next[key];
    onChange(next);
  };

  const handleAddSchemaField = () => {
    if (!schemaKeyToAdd) {
      return;
    }
    if (objectValue[schemaKeyToAdd] !== undefined) {
      return;
    }
    const templateSchema = resolveObjectChildSchema(schema, schemaKeyToAdd, rootSchema);
    const next = { ...objectValue, [schemaKeyToAdd]: getDefaultValue(templateSchema) };
    onChange(next);
    setSchemaKeyToAdd('');
  };

  const handleAddCustomField = () => {
    const normalizedKey = customKey.trim();
    if (!normalizedKey) {
      return;
    }
    if (objectValue[normalizedKey] !== undefined) {
      return;
    }
    const templateSchema =
      resolveObjectChildSchema(schema, normalizedKey, rootSchema) ??
      createSchemaForType(customFieldType);
    const next = { ...objectValue, [normalizedKey]: getDefaultValue(templateSchema) };
    onChange(next);
    setCustomKey('');
    setCustomFieldType('string');
  };

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">{description}</p>
      <div className="space-y-4 rounded-md border p-3">
        {keys.length === 0 && (
          <div className="rounded border border-dashed p-3 text-center text-xs text-muted-foreground">
            尚未设置任何子选项
          </div>
        )}
        {keys.map((key) => {
          const resolvedChildSchema = resolveObjectChildSchema(schema, key, rootSchema);
          const isRequired = requiredKeys.includes(key);
          const childType = getEffectiveType(resolvedChildSchema, objectValue[key]);
          const childIsCompound = isCompoundField(resolvedChildSchema, objectValue[key]);

          return (
            <div key={key} className="space-y-2 rounded-md bg-card p-3">
              <div className="flex flex-col gap-3 md:flex-row md:items-center">
                <div className="flex items-center gap-2 md:basis-1/2">
                  <span className="font-mono text-sm font-semibold">{key}</span>
                  <Badge variant="outline">
                    {getTypeLabel(resolvedChildSchema, objectValue[key])}
                  </Badge>
                </div>
                <div
                  className={`flex items-center gap-3 md:basis-1/2 ${
                    childIsCompound ? 'md:justify-end w-full md:w-auto' : ''
                  }`}
                >
                  {!childIsCompound && (
                    <div
                      className={
                        childType === 'boolean'
                          ? 'flex-1 min-w-0 flex justify-end'
                          : 'flex-1 min-w-0'
                      }
                    >
                      <SchemaField
                        inline
                        schema={resolvedChildSchema}
                        value={objectValue[key]}
                        onChange={(newValue) => handleChildChange(key, newValue)}
                        path={[...path, key]}
                        rootSchema={rootSchema}
                        isRequired={isRequired}
                        showDescription={false}
                        rootValue={rootValue}
                      />
                    </div>
                  )}
                  {!isRequired && (
                    <Button variant="ghost" size="sm" onClick={() => handleDeleteChild(key)}>
                      <Trash2 className="h-4 w-4 text-destructive" />
                    </Button>
                  )}
                </div>
              </div>
              {childIsCompound ? (
                <SchemaField
                  schema={resolvedChildSchema}
                  value={objectValue[key]}
                  onChange={(newValue) => handleChildChange(key, newValue)}
                  path={[...path, key]}
                  rootSchema={rootSchema}
                  isRequired={isRequired}
                  onDelete={!isRequired ? () => handleDeleteChild(key) : undefined}
                  rootValue={rootValue}
                />
              ) : (
                <p className="text-xs text-muted-foreground">
                  {resolvedChildSchema?.description ?? DEFAULT_DESCRIPTION}
                </p>
              )}
            </div>
          );
        })}
        {availableSchemaOptions.length > 0 && (
          <div className="flex flex-col gap-2 md:flex-row md:items-center">
            <p className="text-xs text-muted-foreground md:basis-1/3">
              {isEnvObject ? '选择环境变量' : '从 Schema 添加子选项'}
            </p>
            <div className="flex flex-1 gap-2">
              <Select value={schemaKeyToAdd} onValueChange={setSchemaKeyToAdd}>
                <SelectTrigger>
                  <SelectValue placeholder={isEnvObject ? '选择环境变量' : '选择子选项'} />
                </SelectTrigger>
                <SelectContent>
                  {availableSchemaOptions.map((option) => (
                    <SelectItem key={option.key} value={option.key} className="space-y-1">
                      <div className="flex flex-col text-left">
                        <span className="font-mono text-xs font-semibold">{option.key}</span>
                        <span className="text-[11px] text-muted-foreground">
                          {option.description}
                        </span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button onClick={handleAddSchemaField} disabled={!schemaKeyToAdd}>
                添加
              </Button>
            </div>
          </div>
        )}
        {canAddCustomField && (
          <div className="flex flex-col gap-2 md:flex-row md:items-center">
            <p className="text-xs text-muted-foreground md:basis-1/3">自定义子选项</p>
            <div className="flex flex-1 gap-2 flex-wrap md:flex-nowrap">
              <Input
                className="flex-1"
                value={customKey}
                onChange={(event) => setCustomKey(event.target.value)}
                placeholder="新增子选项名"
              />
              <Select
                value={customFieldType}
                onValueChange={(value) => setCustomFieldType(value as CustomFieldType)}
              >
                <SelectTrigger className="w-28">
                  <SelectValue placeholder="类型" />
                </SelectTrigger>
                <SelectContent>
                  {CUSTOM_FIELD_TYPE_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button onClick={handleAddCustomField} disabled={!customKey.trim()}>
                添加
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function resolveObjectChildSchema(
  schema: JSONSchema,
  key: string,
  rootSchema: JSONSchema | null,
): JSONSchema | undefined {
  if (schema.properties && schema.properties[key]) {
    return resolveSchema(schema.properties[key], rootSchema);
  }

  if (schema.patternProperties) {
    for (const [pattern, patternSchema] of Object.entries(schema.patternProperties)) {
      try {
        const regex = new RegExp(pattern);
        if (regex.test(key)) {
          return resolveSchema(patternSchema, rootSchema);
        }
      } catch {
        // 忽略非法正则
      }
    }
  }

  if (isJsonSchemaObject(schema.additionalProperties)) {
    return resolveSchema(schema.additionalProperties, rootSchema);
  }

  return undefined;
}

function FallbackJsonField({
  value,
  onChange,
  description,
  allowDelete,
  onDelete,
  showDescription = true,
  inline = false,
}: {
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  description: string;
  allowDelete?: boolean;
  onDelete?: () => void;
  showDescription?: boolean;
  inline?: boolean;
}) {
  const input = (
    <Input
      className={inline ? 'flex-1 min-w-0' : 'flex-1 min-w-[220px]'}
      value={safeStringify(value)}
      onChange={(event) => {
        try {
          const parsed = JSON.parse(event.target.value);
          onChange(parsed as JsonValue);
        } catch {
          onChange(event.target.value);
        }
      }}
      placeholder="请输入值或 JSON"
    />
  );

  if (inline) {
    return (
      <div className="flex w-full items-center gap-3 min-w-0">
        {input}
        {allowDelete && onDelete && (
          <Button variant="ghost" size="sm" onClick={onDelete}>
            <Trash2 className="h-4 w-4 text-destructive" />
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-1">
      <div className="flex flex-wrap items-center gap-3">
        {input}
        {allowDelete && onDelete && (
          <Button variant="ghost" size="sm" onClick={onDelete}>
            <Trash2 className="h-4 w-4 text-destructive" />
          </Button>
        )}
      </div>
      {showDescription && <p className="text-xs text-muted-foreground">{description}</p>}
    </div>
  );
}
