import { useRef, useState } from 'react';
import { Upload, FileText } from 'lucide-react';

interface Props {
  onFile: (text: string, filename: string) => void;
  onSample: () => void;
  fileName?: string;
  rowCount?: number;
}

/** Drag-and-drop / click-to-pick CSV upload zone. */
export function UploadZone({ onFile, onSample, fileName, rowCount }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);

  const handleFile = (file: File) => {
    const reader = new FileReader();
    reader.onload = () => onFile(String(reader.result ?? ''), file.name);
    reader.readAsText(file);
  };

  return (
    <div
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        const f = e.dataTransfer.files?.[0];
        if (f) handleFile(f);
      }}
      className={`rounded-xl border-2 border-dashed p-8 text-center transition-colors ${
        dragging ? 'border-blue-500 bg-blue-50' : 'border-slate-300 bg-slate-50'
      }`}
    >
      <input
        ref={inputRef}
        type="file"
        accept=".csv,text/csv,text/plain"
        className="hidden"
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) handleFile(f);
          e.target.value = '';
        }}
      />
      <Upload className="mx-auto mb-3 text-slate-400" size={28} />
      {fileName ? (
        <div className="flex items-center justify-center gap-2 text-slate-700">
          <FileText size={16} className="text-blue-600" />
          <span className="font-medium">{fileName}</span>
          {typeof rowCount === 'number' && (
            <span className="text-slate-500">· {rowCount} rows</span>
          )}
        </div>
      ) : (
        <p className="text-slate-600">
          Drag a <span className="font-semibold">CSV</span> with a date column + a value column,
          or{' '}
          <button
            type="button"
            onClick={() => inputRef.current?.click()}
            className="font-semibold text-blue-600 underline-offset-2 hover:underline"
          >
            browse
          </button>
          .
        </p>
      )}
      <button
        type="button"
        onClick={onSample}
        className="mt-4 text-sm text-slate-500 underline-offset-2 hover:text-slate-700 hover:underline"
      >
        or try a sample dataset
      </button>
    </div>
  );
}
