import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import PlatformDetector from '../../../js/app/utils/PlatformDetector';
import Onboarding from '../../../js/app/onboarding';

export const ssr = false;

export const load: PageLoad = async () => {
    const platformDetector = new PlatformDetector();
    const isMobile = await platformDetector.checkMobile();

    if (isMobile) {
        // On mobile, skip device selection entirely
        const onboarding = new Onboarding();
        await onboarding.initialize();

        // Mark devices step as complete
        await onboarding.completeStep('devices');

        // Get next step and redirect
        const nextStep = onboarding.getNextStep();
        if (nextStep) {
            throw redirect(302, nextStep);
        } else {
            throw redirect(302, '/dashboard');
        }
    }

    return { isMobile };
};
