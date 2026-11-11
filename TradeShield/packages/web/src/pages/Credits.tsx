import { useEffect, useState } from 'react';
import { creditsApi, dashboardApi } from '../lib/api';
import { format } from 'date-fns';

interface CreditBatch {
  id: string;
  org_id: string;
  org_name: string;
  source_org_id: string;
  source_org_name: string;
  period_start: string;
  period_end: string;
  total_mg: number;
  total_eur: number;
  status: string;
  issued_at: string | null;
  created_at: string;
}

interface Device {
  id: string;
  name: string;
  org_id: string;
}

interface Pagination {
  page: number;
  limit: number;
  total: number;
  pages: number;
}

export function Credits() {
  const [credits, setCredits] = useState<CreditBatch[]>([]);
  const [devices, setDevices] = useState<Device[]>([]);
  const [pagination, setPagination] = useState<Pagination | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  // Issue form state
  const [showIssueForm, setShowIssueForm] = useState(false);
  const [issueForm, setIssueForm] = useState({
    source_org_id: '',
    period_start: '',
    period_end: '',
  });
  const [issuing, setIssuing] = useState(false);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);

  useEffect(() => {
    async function fetchData() {
      try {
        const [creditsRes, devicesRes] = await Promise.all([
          creditsApi.list({ page: 1, limit: 20 }),
          dashboardApi.getDevices(),
        ]);
        setCredits(creditsRes.data || []);
        setPagination(creditsRes.pagination);
        setDevices(devicesRes.data || []);
      } catch (err: any) {
        setError(err.message);
      } finally {
        setLoading(false);
      }
    }

    fetchData();
  }, []);

  const handleIssue = async (e: React.FormEvent) => {
    e.preventDefault();
    setIssuing(true);
    
    try {
      await creditsApi.issue(issueForm);
      setShowIssueForm(false);
      setIssueForm({ source_org_id: '', period_start: '', period_end: '' });
      
      // Refresh credits list
      const res = await creditsApi.list({ page: 1, limit: 20 });
      setCredits(res.data || []);
      setPagination(res.pagination);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setIssuing(false);
    }
  };

  const handleDownloadCertificate = async (creditId: string) => {
    setDownloadingId(creditId);
    try {
      await creditsApi.downloadCertificate(creditId);
    } catch (err: any) {
      setError(`Failed to download certificate: ${err.message}`);
    } finally {
      setDownloadingId(null);
    }
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'issued':
        return <span className="px-2 py-1 text-xs rounded-full bg-green-100 text-green-700">Issued</span>;
      case 'pending':
        return <span className="px-2 py-1 text-xs rounded-full bg-yellow-100 text-yellow-700">Pending</span>;
      case 'redeemed':
        return <span className="px-2 py-1 text-xs rounded-full bg-blue-100 text-blue-700">Redeemed</span>;
      default:
        return <span className="px-2 py-1 text-xs rounded-full bg-gray-100 text-gray-700">{status}</span>;
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Credits</h1>
          <p className="text-gray-500 mt-1">Microplastic offset credits</p>
        </div>
        <button
          onClick={() => setShowIssueForm(!showIssueForm)}
          className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700"
        >
          {showIssueForm ? 'Cancel' : 'Issue Credits'}
        </button>
      </div>

      {/* Issue Form */}
      {showIssueForm && (
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Issue New Credits</h2>
          <form onSubmit={handleIssue} className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Source Laundry
                </label>
                <select
                  value={issueForm.source_org_id}
                  onChange={(e) => setIssueForm({ ...issueForm, source_org_id: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500"
                  required
                >
                  <option value="">Select laundry</option>
                  {[...new Set(devices.map(d => d.org_id))].map((orgId) => {
                    const device = devices.find(d => d.org_id === orgId);
                    return (
                      <option key={orgId} value={orgId}>
                        {device?.name || orgId}
                      </option>
                    );
                  })}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Period Start
                </label>
                <input
                  type="date"
                  value={issueForm.period_start}
                  onChange={(e) => setIssueForm({ ...issueForm, period_start: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500"
                  required
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Period End
                </label>
                <input
                  type="date"
                  value={issueForm.period_end}
                  onChange={(e) => setIssueForm({ ...issueForm, period_end: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500"
                  required
                />
              </div>
            </div>
            <button
              type="submit"
              disabled={issuing}
              className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50"
            >
              {issuing ? 'Issuing...' : 'Calculate & Issue'}
            </button>
          </form>
        </div>
      )}

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-xl p-4 text-red-700">
          {error}
        </div>
      )}

      {/* Credits Table */}
      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
        {loading ? (
          <div className="p-12 text-center text-gray-500">Loading credits...</div>
        ) : credits.length === 0 ? (
          <div className="p-12 text-center">
            <div className="text-4xl mb-4">💰</div>
            <h3 className="text-lg font-medium text-gray-900">No credits yet</h3>
            <p className="text-gray-500 mt-2">Issue credits for captured microplastic to get started</p>
          </div>
        ) : (
          <>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="bg-gray-50 border-b border-gray-200">
                    <th className="text-left py-3 px-4 font-medium text-gray-500">ID</th>
                    <th className="text-left py-3 px-4 font-medium text-gray-500">Period</th>
                    <th className="text-left py-3 px-4 font-medium text-gray-500">From</th>
                    <th className="text-left py-3 px-4 font-medium text-gray-500">Captured</th>
                    <th className="text-left py-3 px-4 font-medium text-gray-500">Value</th>
                    <th className="text-left py-3 px-4 font-medium text-gray-500">Status</th>
                    <th className="text-left py-3 px-4 font-medium text-gray-500">Issued</th>
                    <th className="text-left py-3 px-4 font-medium text-gray-500"></th>
                  </tr>
                </thead>
                <tbody>
                  {credits.map((credit) => (
                    <tr key={credit.id} className="border-b border-gray-100 hover:bg-gray-50">
                      <td className="py-3 px-4">
                        <span className="font-mono text-xs bg-gray-100 px-2 py-1 rounded">
                          {credit.id.slice(0, 8)}...
                        </span>
                      </td>
                      <td className="py-3 px-4">
                        {format(new Date(credit.period_start), 'MMM d')} -{' '}
                        {format(new Date(credit.period_end), 'MMM d, yyyy')}
                      </td>
                      <td className="py-3 px-4 font-medium">
                        {credit.total_mg.toFixed(1)} mg
                      </td>
                      <td className="py-3 px-4 font-semibold text-green-600">
                        €{credit.total_eur.toFixed(2)}
                      </td>
                      <td className="py-3 px-4">
                        {getStatusBadge(credit.status)}
                      </td>
                      <td className="py-3 px-4 text-gray-500">
                        {credit.issued_at ? format(new Date(credit.issued_at), 'MMM d, yyyy') : '-'}
                      </td>
                      <td className="py-3 px-4">
                        <button
                          onClick={() => handleDownloadCertificate(credit.id)}
                          disabled={downloadingId === credit.id}
                          className="px-3 py-1.5 text-xs font-medium text-primary-600 bg-primary-50 rounded-lg hover:bg-primary-100 disabled:opacity-50 flex items-center gap-1.5"
                        >
                          {downloadingId === credit.id ? (
                            <svg className="animate-spin h-3 w-3" viewBox="0 0 24 24"><circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" /><path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" /></svg>
                          ) : (
                            <svg className="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
                          )}
                          PDF
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Pagination */}
            {pagination && pagination.pages > 1 && (
              <div className="p-4 border-t border-gray-200 flex items-center justify-between">
                <p className="text-sm text-gray-500">
                  Page {pagination.page} of {pagination.pages}
                </p>
                <div className="flex gap-2">
                  <button
                    onClick={() => {
                      /* TODO: implement page change */
                    }}
                    disabled={pagination.page === 1}
                    className="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-50"
                  >
                    Previous
                  </button>
                  <button
                    onClick={() => {
                      /* TODO: implement page change */
                    }}
                    disabled={pagination.page === pagination.pages}
                    className="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-50"
                  >
                    Next
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
