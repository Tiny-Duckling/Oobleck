from django.utils import timezone
from rest_framework import viewsets

from .mixins import CreateUpdateUserMixin, CreateUserMixin
from .models import Opening, Application
from .permissions import IsStaffOrReadOnly, IsAuthenticatedAndOwner
from .serializers import OpeningSerializer, ApplicationSerializer


class OpeningViewSet(CreateUpdateUserMixin, viewsets.ModelViewSet):
    queryset = Opening.objects.filter(deadline__gte=timezone.now())
    serializer_class = OpeningSerializer
    permission_classes = [IsStaffOrReadOnly]


class ApplicationViewSet(CreateUserMixin, viewsets.ModelViewSet):
    queryset = Application.objects.all()
    serializer_class = ApplicationSerializer
    permission_classes = [IsAuthenticatedAndOwner]
