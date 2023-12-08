import { ComponentFixture, TestBed } from '@angular/core/testing';

import { IconLasrComponent } from './icon-lasr.component';

describe('IconTudComponent', () => {
  let component: IconLasrComponent;
  let fixture: ComponentFixture<IconLasrComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [IconLasrComponent]
    });
    fixture = TestBed.createComponent(IconLasrComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
