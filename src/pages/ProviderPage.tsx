import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from '../components/common/Toast';
import { Modal } from '../components/common/Modal';
import { Button } from '../components/common/Button';
import { ProviderList } from '../components/Providers/ProviderList';
import { ProviderStatus } from '../components/Models/ProviderStatus';
import { ApiKeyModal } from '../components/Providers/ApiKeyModal';
import { CustomProviderModal } from '../components/Providers/CustomProviderModal';
import { KeyRound, Wifi, Settings, RefreshCw } from 'lucide-react';
import { cn } from '../components/common/cn';
import { usePreloadStore } from '../store/preloadStore';
import { deleteProviderAuth, type ProviderInfo } from '../services/tauri';

type TabType = 'status' | 'config';

export function ProviderPage() {
  const { t } = useTranslation();
  const refreshModels = usePreloadStore((s) => s.refreshModels);
  const providers = usePreloadStore((s) => s.providerCatalog.data) || [];
  const providerCatalogRefreshing = usePreloadStore((s) => s.providerCatalog.refreshing);
  const providerCatalogError = usePreloadStore((s) => s.providerCatalog.error);
  const refreshProviderCatalog = usePreloadStore((s) => s.refreshProviderCatalog);
  const [activeTab, setActiveTab] = useState<TabType>('status');
  const [isStatusRefreshing, setIsStatusRefreshing] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<ProviderInfo | null>(null);
  const [showCustomModal, setShowCustomModal] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<ProviderInfo | null>(null);

  useEffect(() => {
    void refreshProviderCatalog();
  }, [refreshProviderCatalog]);

  const handleConfigure = (provider: ProviderInfo) => {
    setSelectedProvider(provider);
  };

  const handleEdit = (provider: ProviderInfo) => {
    setSelectedProvider(provider);
  };

  const handleDelete = (provider: ProviderInfo) => {
    setDeleteConfirm(provider);
  };

  const handleDeleteConfirm = async () => {
    if (!deleteConfirm) return;
    try {
      await deleteProviderAuth(deleteConfirm.id);
      toast.success(t('provider.deleteSuccess'));
      await Promise.all([
        refreshProviderCatalog(true),
        refreshModels(true),
      ]);
    } catch (err) {
      toast.error(t('provider.deleteFailed'));
      console.error('Failed to delete provider auth:', err);
    } finally {
      setDeleteConfirm(null);
    }
  };

  const handleAddCustom = () => {
    setShowCustomModal(true);
  };

  const handleSuccess = async () => {
    await Promise.allSettled([
      refreshProviderCatalog(true),
      refreshModels(true),
    ]);
  };

  const handleRefresh = async () => {
    if (activeTab === 'status') {
      setIsStatusRefreshing(true);
      try {
        await refreshModels(true);
      } catch (err) {
        toast.error(t('provider.loadFailed'));
        console.error('Failed to refresh provider status models:', err);
      } finally {
        setIsStatusRefreshing(false);
      }
      return;
    }

    await refreshProviderCatalog(true);
  };

  const configuredProviders = providers.filter(p => p.is_configured);
  const unconfiguredBuiltinProviders = providers.filter(
    p => !p.is_configured && p.is_builtin
  );
  const unconfiguredCustomProviders = providers.filter(
    p => !p.is_configured && !p.is_builtin
  );

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center gap-4 p-6 bg-gradient-to-r from-indigo-50 to-purple-50 rounded-2xl border border-indigo-100 mb-6">
        <div className="w-12 h-12 bg-indigo-600 rounded-xl flex items-center justify-center shadow-lg shadow-indigo-200">
          <KeyRound className="w-6 h-6 text-white" />
        </div>
        <div className="flex-1">
          <h2 className="text-xl font-semibold text-slate-800">{t('provider.title')}</h2>
          <p className="text-slate-600 mt-1">{t('provider.description')}</p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleRefresh}
          disabled={isStatusRefreshing || providerCatalogRefreshing}
        >
          <RefreshCw
            className={cn(
              'w-4 h-4 mr-2',
              (isStatusRefreshing || providerCatalogRefreshing) && 'animate-spin'
            )}
          />
          {t('common.refresh')}
        </Button>
      </div>

      <div className="flex items-center gap-2 p-1 bg-slate-100 rounded-xl w-fit mb-6">
        <button
          onClick={() => setActiveTab('status')}
          className={cn(
            'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all',
            activeTab === 'status'
              ? 'bg-white text-indigo-700 shadow-sm border border-slate-200'
              : 'text-slate-600 hover:text-slate-800 hover:bg-slate-200/50'
          )}
        >
          <Wifi className="w-4 h-4" />
          {t('provider.tabs.status')}
        </button>
        <button
          onClick={() => setActiveTab('config')}
          className={cn(
            'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all',
            activeTab === 'config'
              ? 'bg-white text-indigo-700 shadow-sm border border-slate-200'
              : 'text-slate-600 hover:text-slate-800 hover:bg-slate-200/50'
          )}
        >
          <Settings className="w-4 h-4" />
          {t('provider.tabs.config')}
        </button>
      </div>

      {activeTab === 'status' ? (
        <ProviderStatus />
      ) : (
        <>
          {providerCatalogRefreshing && providers.length > 0 && (
            <div className="mb-4 rounded-lg border border-indigo-100 bg-indigo-50 px-4 py-2 text-sm text-indigo-700">
              {t('common.loading')}
            </div>
          )}
          {providers.length === 0 && providerCatalogRefreshing ? (
            <div className="space-y-4 animate-pulse">
              <div className="h-12 bg-slate-200 rounded-xl" />
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {[1, 2, 3, 4].map(i => (
                  <div key={i} className="h-32 bg-slate-200 rounded-xl" />
                ))}
              </div>
            </div>
          ) : providers.length === 0 && providerCatalogError ? (
            <div className="py-12 text-center">
              <p className="mb-4 text-sm text-rose-600">{t('provider.loadFailed')}</p>
              <Button variant="secondary" onClick={() => refreshProviderCatalog(true)}>
                <RefreshCw className="mr-2 h-4 w-4" />
                {t('common.retry')}
              </Button>
            </div>
          ) : (
            <ProviderList
              configuredProviders={configuredProviders}
              unconfiguredBuiltinProviders={unconfiguredBuiltinProviders}
              unconfiguredCustomProviders={unconfiguredCustomProviders}
              onConfigure={handleConfigure}
              onEdit={handleEdit}
              onDelete={handleDelete}
              onAddCustom={handleAddCustom}
            />
          )}
        </>
      )}

      {selectedProvider && (
        <ApiKeyModal
          provider={selectedProvider}
          onClose={() => setSelectedProvider(null)}
          onSuccess={handleSuccess}
        />
      )}

      {showCustomModal && (
        <CustomProviderModal
          onClose={() => setShowCustomModal(false)}
          onSuccess={handleSuccess}
        />
      )}

      <Modal
        isOpen={deleteConfirm !== null}
        onClose={() => setDeleteConfirm(null)}
        title={t('provider.confirmDelete')}
        size="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setDeleteConfirm(null)}>
              {t('button.cancel')}
            </Button>
            <Button
              variant="primary"
              onClick={handleDeleteConfirm}
              className="bg-red-500 hover:bg-red-600"
            >
              {t('button.delete')}
            </Button>
          </>
        }
      >
        <div className="space-y-2">
          <p className="text-slate-700">
            {deleteConfirm && t('provider.confirmDeleteMessage', { name: deleteConfirm.name })}
          </p>
          <p className="text-sm text-slate-500">
            {t('provider.confirmDeleteHint')}
          </p>
        </div>
      </Modal>
    </div>
  );
}
