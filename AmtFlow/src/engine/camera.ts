import { Camera, CameraResultType, CameraSource } from '@capacitor/camera';
import { Haptics, ImpactStyle } from '@capacitor/haptics';

export async function takePhoto(): Promise<string | null> {
  try {
    try {
      await Haptics.impact({ style: ImpactStyle.Light });
    } catch (_) {}

    const image = await Camera.getPhoto({
      quality: 90,
      allowEditing: false,
      resultType: CameraResultType.DataUrl,
      source: CameraSource.Camera
    });

    return image.dataUrl || null;
  } catch (err) {
    console.warn('Native camera cancelled or failed, fallback to file picker:', err);
    return null;
  }
}

export async function pickPhotoFromGallery(): Promise<string | null> {
  try {
    const image = await Camera.getPhoto({
      quality: 90,
      allowEditing: false,
      resultType: CameraResultType.DataUrl,
      source: CameraSource.Photos
    });

    return image.dataUrl || null;
  } catch (err) {
    console.warn('Gallery picker cancelled or failed:', err);
    return null;
  }
}
