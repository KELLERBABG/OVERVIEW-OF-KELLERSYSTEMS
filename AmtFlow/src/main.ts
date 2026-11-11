// @ts-ignore
import { classify } from './engine/classifier';
// @ts-ignore
import { idbPut, idbAll, idbDelete, idbClear, compressImage } from './storage/db';
// @ts-ignore
import { loadLocale, getCurrentLocale, setCurrentLocale } from './engine/i18n';
// @ts-ignore
import { takePhoto, pickPhotoFromGallery } from './engine/camera';

console.log('AmtFlow Native Mobile App Engine initialized.');

// Export to window for seamless UI bindings
(window as any).AmtFlowApp = {
  classify,
  idbPut,
  idbAll,
  idbDelete,
  idbClear,
  compressImage,
  loadLocale,
  getCurrentLocale,
  setCurrentLocale,
  takePhoto,
  pickPhotoFromGallery
};
